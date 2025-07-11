//! FFmpeg-based decoder implementation for CCPlayer
//! 
//! Provides video and audio decoding using the ffmpeg-next crate with
//! support for hardware acceleration and various codecs.

use crate::decoder::{
    AudioSamples, AudioStreamInfo, ColorSpace, Decoder, HdrMetadata, HwAccelMethod,
    MasteringDisplay, MediaInfo, MediaMetadata, SubtitleStreamInfo, VideoStreamInfo,
};
use crate::renderer::{FrameData, VideoFrame};
use crate::utils::error::{CCPlayerError, Result};
use ffmpeg_next as ffmpeg;
use ffmpeg_next::{format, media, util};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use super::frame_queue::FrameQueue;
use super::hw_accel::{HardwareAccelerator, HwAccelConfig};
use super::stream_info::StreamInfoExtractor;

/// FFmpeg decoder implementation
pub struct FFmpegDecoder {
    /// Input format context
    input_context: Option<format::context::Input>,
    
    /// Video decoder
    video_decoder: Option<VideoDecoder>,
    
    /// Audio decoder
    audio_decoder: Option<AudioDecoder>,
    
    /// Frame queue for buffering
    frame_queue: Arc<Mutex<FrameQueue>>,
    
    /// Hardware accelerator
    hw_accelerator: Option<Box<dyn HardwareAccelerator>>,
    
    /// Stream information
    media_info: Option<MediaInfo>,
    
    /// Current playback position
    position: Duration,
    
    /// End of file reached
    eof: bool,
    
    /// Hardware acceleration enabled
    hw_accel_enabled: bool,
}

/// Video decoder state
struct VideoDecoder {
    /// Decoder context
    decoder: ffmpeg::codec::decoder::Video,
    
    /// Stream index
    stream_index: usize,
    
    /// Time base for PTS conversion
    time_base: ffmpeg::Rational,
    
    /// Frame converter for pixel format conversion
    converter: Option<ffmpeg::software::scaling::Context>,
    
    /// Target pixel format
    target_format: ffmpeg::format::Pixel,
}

/// Audio decoder state
struct AudioDecoder {
    /// Decoder context
    decoder: ffmpeg::codec::decoder::Audio,
    
    /// Stream index
    stream_index: usize,
    
    /// Time base for PTS conversion
    time_base: ffmpeg::Rational,
    
    /// Audio resampler
    resampler: Option<ffmpeg::software::resampling::Context>,
    
    /// Target sample format
    target_format: ffmpeg::format::Sample,
    
    /// Target sample rate
    target_rate: u32,
    
    /// Target channel layout
    target_layout: ffmpeg::channel_layout::ChannelLayout,
}

impl FFmpegDecoder {
    /// Initialize FFmpeg library
    fn init_ffmpeg() {
        ffmpeg::init().unwrap();
        
        // Set log level
        ffmpeg::log::set_level(ffmpeg::log::Level::Warning);
    }
    
    /// Open video stream and create decoder
    fn open_video_stream(
        &mut self,
        input: &mut format::context::Input,
        hw_accel: Option<&Box<dyn HardwareAccelerator>>,
    ) -> Result<()> {
        // Find video stream
        let stream = input
            .streams()
            .best(media::Type::Video)
            .ok_or_else(|| CCPlayerError::decoder_error("No video stream found"))?;
        
        let stream_index = stream.index();
        let time_base = stream.time_base();
        
        // Get codec parameters
        let codec_params = stream.parameters();
        
        // Find decoder
        let codec = ffmpeg::codec::decoder::find(codec_params.id())
            .ok_or_else(|| CCPlayerError::decoder_error("Video codec not found"))?;
        
        // Create decoder context
        let mut context = ffmpeg::codec::context::Context::from_parameters(codec_params)?;
        context.set_threading(ffmpeg::codec::threading::Config {
            kind: ffmpeg::codec::threading::Type::Frame,
            count: 0, // Auto-detect
        });
        
        // Configure hardware acceleration if available
        if let Some(hw_accel) = hw_accel {
            hw_accel.configure_context(&mut context)?;
        }
        
        // Open decoder
        let decoder = context.decoder().video()?;
        
        // Determine target pixel format
        let target_format = if cfg!(target_os = "windows") {
            ffmpeg::format::Pixel::NV12
        } else {
            ffmpeg::format::Pixel::YUV420P
        };
        
        self.video_decoder = Some(VideoDecoder {
            decoder,
            stream_index,
            time_base,
            converter: None,
            target_format,
        });
        
        Ok(())
    }
    
    /// Open audio stream and create decoder
    fn open_audio_stream(&mut self, input: &mut format::context::Input) -> Result<()> {
        // Find audio stream
        let stream = match input.streams().best(media::Type::Audio) {
            Some(s) => s,
            None => return Ok(()), // No audio stream is OK
        };
        
        let stream_index = stream.index();
        let time_base = stream.time_base();
        
        // Get codec parameters
        let codec_params = stream.parameters();
        
        // Find decoder
        let codec = ffmpeg::codec::decoder::find(codec_params.id())
            .ok_or_else(|| CCPlayerError::decoder_error("Audio codec not found".to_string()))?;
        
        // Create decoder context
        let context = ffmpeg::codec::context::Context::from_parameters(codec_params)?;
        
        // Open decoder
        let decoder = context.decoder().audio()?;
        
        // Target audio format: 32-bit float, stereo, 48kHz
        let target_format = ffmpeg::format::Sample::F32(ffmpeg::format::sample::Type::Planar);
        let target_rate = 48000;
        let target_layout = ffmpeg::channel_layout::ChannelLayout::STEREO;
        
        self.audio_decoder = Some(AudioDecoder {
            decoder,
            stream_index,
            time_base,
            resampler: None,
            target_format,
            target_rate,
            target_layout,
        });
        
        Ok(())
    }
    
    /// Convert FFmpeg frame to our VideoFrame format
    fn convert_video_frame(&mut self, frame: &ffmpeg::frame::Video) -> Result<VideoFrame> {
        let video_decoder = self.video_decoder.as_mut()
            .ok_or_else(|| CCPlayerError::decoder_error("No video decoder".to_string()))?;
        
        // Calculate PTS in microseconds
        let pts = if frame.timestamp().is_some() {
            let pts_seconds = frame.timestamp().unwrap() as f64 * 
                video_decoder.time_base.numerator() as f64 / 
                video_decoder.time_base.denominator() as f64;
            (pts_seconds * 1_000_000.0) as i64
        } else {
            0
        };
        
        // Get frame duration
        let duration = if frame.duration() > 0 {
            let duration_seconds = frame.duration() as f64 *
                video_decoder.time_base.numerator() as f64 /
                video_decoder.time_base.denominator() as f64;
            (duration_seconds * 1_000_000.0) as i64
        } else {
            16667 // Default to ~60fps
        };
        
        // Convert pixel format if needed
        let converted_frame = if frame.format() != video_decoder.target_format {
            // Create or update converter
            if video_decoder.converter.is_none() ||
               video_decoder.converter.as_ref().unwrap().input().width != frame.width() ||
               video_decoder.converter.as_ref().unwrap().input().height != frame.height() {
                
                video_decoder.converter = Some(
                    ffmpeg::software::scaling::Context::get(
                        frame.format(),
                        frame.width(),
                        frame.height(),
                        video_decoder.target_format,
                        frame.width(),
                        frame.height(),
                        ffmpeg::software::scaling::Flags::BILINEAR,
                    )?
                );
            }
            
            let mut converted = ffmpeg::frame::Video::empty();
            video_decoder.converter.as_mut().unwrap().run(frame, &mut converted)?;
            converted
        } else {
            frame.clone()
        };
        
        // Extract frame data based on pixel format
        let frame_data = match converted_frame.format() {
            ffmpeg::format::Pixel::YUV420P => {
                let y_plane = converted_frame.data(0).to_vec();
                let u_plane = converted_frame.data(1).to_vec();
                let v_plane = converted_frame.data(2).to_vec();
                let y_stride = converted_frame.stride(0);
                let uv_stride = converted_frame.stride(1);
                
                FrameData::Yuv420 {
                    y_plane,
                    u_plane,
                    v_plane,
                    y_stride,
                    uv_stride,
                }
            }
            ffmpeg::format::Pixel::NV12 => {
                let y_plane = converted_frame.data(0).to_vec();
                let uv_plane = converted_frame.data(1).to_vec();
                let y_stride = converted_frame.stride(0);
                let uv_stride = converted_frame.stride(1);
                
                FrameData::Nv12 {
                    y_plane,
                    uv_plane,
                    y_stride,
                    uv_stride,
                }
            }
            _ => {
                // Convert to RGB as fallback
                let mut rgb_converter = ffmpeg::software::scaling::Context::get(
                    converted_frame.format(),
                    converted_frame.width(),
                    converted_frame.height(),
                    ffmpeg::format::Pixel::RGB24,
                    converted_frame.width(),
                    converted_frame.height(),
                    ffmpeg::software::scaling::Flags::BILINEAR,
                )?;
                
                let mut rgb_frame = ffmpeg::frame::Video::empty();
                rgb_converter.run(&converted_frame, &mut rgb_frame)?;
                
                FrameData::Rgb {
                    data: rgb_frame.data(0).to_vec(),
                    stride: rgb_frame.stride(0),
                }
            }
        };
        
        Ok(VideoFrame {
            data: frame_data,
            pts,
            duration,
            width: converted_frame.width(),
            height: converted_frame.height(),
            par: 1.0, // TODO: Extract proper PAR from stream
        })
    }
    
    /// Convert FFmpeg audio frame to our AudioSamples format
    fn convert_audio_frame(&mut self, frame: &ffmpeg::frame::Audio) -> Result<AudioSamples> {
        let audio_decoder = self.audio_decoder.as_mut()
            .ok_or_else(|| CCPlayerError::decoder_error("No audio decoder".to_string()))?;
        
        // Calculate PTS
        let pts = if frame.timestamp().is_some() {
            let pts_seconds = frame.timestamp().unwrap() as f64 * 
                audio_decoder.time_base.numerator() as f64 / 
                audio_decoder.time_base.denominator() as f64;
            (pts_seconds * 1_000_000.0) as i64
        } else {
            0
        };
        
        // Create resampler if needed
        if audio_decoder.resampler.is_none() ||
           frame.rate() != audio_decoder.target_rate ||
           frame.format() != audio_decoder.target_format ||
           frame.channel_layout() != audio_decoder.target_layout {
            
            audio_decoder.resampler = Some(
                ffmpeg::software::resampling::Context::get(
                    frame.format(),
                    frame.channel_layout(),
                    frame.rate(),
                    audio_decoder.target_format,
                    audio_decoder.target_layout,
                    audio_decoder.target_rate,
                )?
            );
        }
        
        // Resample audio
        let mut resampled = ffmpeg::frame::Audio::empty();
        let delay = audio_decoder.resampler.as_ref().unwrap()
            .run(frame, &mut resampled)?
            .unwrap_or(0);
        
        // Convert to f32 samples
        let sample_count = resampled.samples();
        let channels = resampled.channel_layout().channels() as usize;
        let mut data = Vec::with_capacity(sample_count * channels);
        
        // Extract samples (assuming planar f32 format)
        for sample_idx in 0..sample_count {
            for channel in 0..channels {
                let plane = resampled.plane::<f32>(channel);
                data.push(plane[sample_idx]);
            }
        }
        
        Ok(AudioSamples {
            data,
            sample_count,
            channels,
            sample_rate: audio_decoder.target_rate,
            pts,
        })
    }
}

impl Decoder for FFmpegDecoder {
    fn new() -> Result<Self> {
        Self::init_ffmpeg();
        
        Ok(Self {
            input_context: None,
            video_decoder: None,
            audio_decoder: None,
            frame_queue: Arc::new(Mutex::new(FrameQueue::new(30))), // 30 frame buffer
            hw_accelerator: None,
            media_info: None,
            position: Duration::ZERO,
            eof: false,
            hw_accel_enabled: true,
        })
    }
    
    fn open_file(&mut self, path: &Path) -> Result<MediaInfo> {
        // Open input file
        let mut input = format::input(path)?;
        
        // Extract media information
        let extractor = StreamInfoExtractor::new();
        let media_info = extractor.extract_info(&mut input, path.to_string_lossy())?;
        
        // Setup hardware acceleration if enabled
        if self.hw_accel_enabled {
            let hw_config = HwAccelConfig::detect_best_method(&media_info)?;
            if let Some(config) = hw_config {
                self.hw_accelerator = Some(super::hw_accel::create_accelerator(config)?);
            }
        }
        
        // Open video stream
        self.open_video_stream(&mut input, self.hw_accelerator.as_ref())?;
        
        // Open audio stream
        self.open_audio_stream(&mut input)?;
        
        self.input_context = Some(input);
        self.media_info = Some(media_info.clone());
        self.eof = false;
        self.position = Duration::ZERO;
        
        Ok(media_info)
    }
    
    fn open_url(&mut self, url: &str) -> Result<MediaInfo> {
        // Open input URL
        let mut options = ffmpeg::Dictionary::new();
        options.set("rtsp_transport", "tcp");
        options.set("buffer_size", "1048576"); // 1MB buffer
        
        let mut input = format::input_with_dictionary(&url, options)?;
        
        // Extract media information
        let extractor = StreamInfoExtractor::new();
        let media_info = extractor.extract_info(&mut input, url)?;
        
        // Setup hardware acceleration if enabled
        if self.hw_accel_enabled {
            let hw_config = HwAccelConfig::detect_best_method(&media_info)?;
            if let Some(config) = hw_config {
                self.hw_accelerator = Some(super::hw_accel::create_accelerator(config)?);
            }
        }
        
        // Open video stream
        self.open_video_stream(&mut input, self.hw_accelerator.as_ref())?;
        
        // Open audio stream
        self.open_audio_stream(&mut input)?;
        
        self.input_context = Some(input);
        self.media_info = Some(media_info.clone());
        self.eof = false;
        self.position = Duration::ZERO;
        
        Ok(media_info)
    }
    
    fn decode_frame(&mut self) -> Result<Option<VideoFrame>> {
        if self.eof {
            return Ok(None);
        }
        
        let input = self.input_context.as_mut()
            .ok_or_else(|| CCPlayerError::decoder_error("No input context".to_string()))?;
        
        let video_decoder = self.video_decoder.as_mut()
            .ok_or_else(|| CCPlayerError::decoder_error("No video decoder".to_string()))?;
        
        // Try to get frame from queue first
        {
            let mut queue = self.frame_queue.lock();
            if let Some(frame) = queue.pop_frame() {
                self.position = Duration::from_micros(frame.pts as u64);
                return Ok(Some(frame));
            }
        }
        
        // Decode new frames
        loop {
            match input.packets().next() {
                Some((stream, packet)) => {
                    if stream.index() == video_decoder.stream_index {
                        // Send packet to decoder
                        video_decoder.decoder.send_packet(&packet)?;
                        
                        // Receive frames
                        let mut decoded_frame = ffmpeg::frame::Video::empty();
                        while video_decoder.decoder.receive_frame(&mut decoded_frame).is_ok() {
                            let frame = self.convert_video_frame(&decoded_frame)?;
                            
                            // Update position
                            self.position = Duration::from_micros(frame.pts as u64);
                            
                            // Add to queue or return directly
                            let mut queue = self.frame_queue.lock();
                            if queue.is_empty() {
                                return Ok(Some(frame));
                            } else {
                                queue.push_frame(frame)?;
                            }
                        }
                    } else if let Some(audio_decoder) = &self.audio_decoder {
                        if stream.index() == audio_decoder.stream_index {
                            // Handle audio packet (for now just decode to keep sync)
                            // TODO: Properly handle audio samples
                        }
                    }
                }
                None => {
                    // End of stream
                    self.eof = true;
                    
                    // Flush decoder
                    video_decoder.decoder.send_eof()?;
                    
                    let mut decoded_frame = ffmpeg::frame::Video::empty();
                    if video_decoder.decoder.receive_frame(&mut decoded_frame).is_ok() {
                        let frame = self.convert_video_frame(&decoded_frame)?;
                        self.position = Duration::from_micros(frame.pts as u64);
                        return Ok(Some(frame));
                    }
                    
                    return Ok(None);
                }
            }
        }
    }
    
    fn decode_audio(&mut self) -> Result<Option<AudioSamples>> {
        if self.eof {
            return Ok(None);
        }
        
        let input = self.input_context.as_mut()
            .ok_or_else(|| CCPlayerError::decoder_error("No input context".to_string()))?;
        
        let audio_decoder = self.audio_decoder.as_mut()
            .ok_or_else(|| CCPlayerError::decoder_error("No audio decoder".to_string()))?;
        
        // Decode audio frames
        loop {
            match input.packets().next() {
                Some((stream, packet)) => {
                    if stream.index() == audio_decoder.stream_index {
                        // Send packet to decoder
                        audio_decoder.decoder.send_packet(&packet)?;
                        
                        // Receive frames
                        let mut decoded_frame = ffmpeg::frame::Audio::empty();
                        if audio_decoder.decoder.receive_frame(&mut decoded_frame).is_ok() {
                            return Ok(Some(self.convert_audio_frame(&decoded_frame)?));
                        }
                    }
                }
                None => {
                    // End of stream
                    self.eof = true;
                    
                    // Flush decoder
                    audio_decoder.decoder.send_eof()?;
                    
                    let mut decoded_frame = ffmpeg::frame::Audio::empty();
                    if audio_decoder.decoder.receive_frame(&mut decoded_frame).is_ok() {
                        return Ok(Some(self.convert_audio_frame(&decoded_frame)?));
                    }
                    
                    return Ok(None);
                }
            }
        }
    }
    
    fn seek(&mut self, timestamp: Duration) -> Result<()> {
        let input = self.input_context.as_mut()
            .ok_or_else(|| CCPlayerError::decoder_error("No input context".to_string()))?;
        
        let video_decoder = self.video_decoder.as_ref()
            .ok_or_else(|| CCPlayerError::decoder_error("No video decoder".to_string()))?;
        
        // Convert timestamp to stream time base
        let stream_timestamp = (timestamp.as_secs_f64() * 
            video_decoder.time_base.denominator() as f64 / 
            video_decoder.time_base.numerator() as f64) as i64;
        
        // Seek to timestamp
        input.seek(stream_timestamp, stream_timestamp..stream_timestamp)?;
        
        // Flush decoders
        self.flush()?;
        
        // Clear frame queue
        self.frame_queue.lock().clear();
        
        self.position = timestamp;
        self.eof = false;
        
        Ok(())
    }
    
    fn position(&self) -> Duration {
        self.position
    }
    
    fn is_eof(&self) -> bool {
        self.eof && self.frame_queue.lock().is_empty()
    }
    
    fn flush(&mut self) -> Result<()> {
        if let Some(video_decoder) = &mut self.video_decoder {
            video_decoder.decoder.flush();
        }
        
        if let Some(audio_decoder) = &mut self.audio_decoder {
            audio_decoder.decoder.flush();
        }
        
        self.frame_queue.lock().clear();
        
        Ok(())
    }
    
    fn set_hardware_acceleration(&mut self, enabled: bool) -> Result<()> {
        self.hw_accel_enabled = enabled;
        
        // If we have an open file, we need to reopen it with new settings
        if let Some(media_info) = &self.media_info {
            let source = media_info.source.clone();
            
            // Close current context
            self.input_context = None;
            self.video_decoder = None;
            self.audio_decoder = None;
            self.hw_accelerator = None;
            
            // Reopen with new settings
            if source.starts_with("http://") || source.starts_with("https://") || 
               source.starts_with("rtmp://") || source.starts_with("rtsp://") {
                self.open_url(&source)?;
            } else {
                self.open_file(Path::new(&source))?;
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ffmpeg_decoder_creation() {
        let decoder = FFmpegDecoder::new();
        assert!(decoder.is_ok());
        
        let decoder = decoder.unwrap();
        assert!(decoder.input_context.is_none());
        assert!(decoder.video_decoder.is_none());
        assert!(decoder.audio_decoder.is_none());
        assert_eq!(decoder.position, Duration::ZERO);
        assert!(!decoder.eof);
    }
    
    #[test]
    fn test_initial_state() {
        let decoder = FFmpegDecoder::new().unwrap();
        assert_eq!(decoder.position(), Duration::ZERO);
        assert!(!decoder.is_eof());
    }
}