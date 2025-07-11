//! Media stream information extraction
//! 
//! Provides utilities for extracting detailed information about media streams
//! including video, audio, and subtitle tracks with metadata.

use crate::decoder::{
    AudioStreamInfo, ColorSpace, HdrMetadata, MasteringDisplay, MediaInfo, 
    MediaMetadata, SubtitleStreamInfo, VideoStreamInfo,
};
use crate::utils::error::{CCPlayerError, Result};
use ffmpeg_next as ffmpeg;
use std::collections::HashMap;
use std::time::Duration;

/// Stream information extractor
pub struct StreamInfoExtractor {
    /// Metadata parser
    metadata_parser: MetadataParser,
}

impl StreamInfoExtractor {
    /// Create a new stream info extractor
    pub fn new() -> Self {
        Self {
            metadata_parser: MetadataParser::new(),
        }
    }
    
    /// Extract media information from input context
    pub fn extract_info(
        &self,
        input: &mut ffmpeg::format::context::Input,
        source: String,
    ) -> Result<MediaInfo> {
        // Get format info
        let format = input.format().name().to_string();
        let duration = Duration::from_secs_f64(input.duration() as f64 / ffmpeg::ffi::AV_TIME_BASE as f64);
        let bitrate = if input.bit_rate() > 0 {
            Some(input.bit_rate() as u32)
        } else {
            None
        };
        let file_size = input.size().map(|s| s as u64);
        
        // Extract metadata
        let metadata = self.metadata_parser.parse_metadata(input.metadata());
        
        // Extract stream information
        let mut video_streams = Vec::new();
        let mut audio_streams = Vec::new();
        let mut subtitle_streams = Vec::new();
        
        for stream in input.streams() {
            match stream.parameters().medium() {
                ffmpeg::media::Type::Video => {
                    if let Some(info) = self.extract_video_stream_info(stream) {
                        video_streams.push(info);
                    }
                }
                ffmpeg::media::Type::Audio => {
                    if let Some(info) = self.extract_audio_stream_info(stream) {
                        audio_streams.push(info);
                    }
                }
                ffmpeg::media::Type::Subtitle => {
                    if let Some(info) = self.extract_subtitle_stream_info(stream) {
                        subtitle_streams.push(info);
                    }
                }
                _ => {}
            }
        }
        
        Ok(MediaInfo {
            source,
            duration,
            video_streams,
            audio_streams,
            subtitle_streams,
            format,
            file_size,
            bitrate,
            metadata,
        })
    }
    
    /// Extract video stream information
    fn extract_video_stream_info(&self, stream: ffmpeg::format::stream::Stream) -> Option<VideoStreamInfo> {
        let params = stream.parameters();
        let codec_params = params.as_video().ok()?;
        
        let index = stream.index();
        let codec = ffmpeg::codec::decoder::find(params.id())
            .map(|c| c.name().to_string())
            .unwrap_or_else(|| format!("unknown ({})", params.id().name()));
        
        let width = codec_params.width();
        let height = codec_params.height();
        
        // Calculate frame rate
        let fps = if stream.avg_frame_rate().denominator() != 0 {
            stream.avg_frame_rate().numerator() as f32 / stream.avg_frame_rate().denominator() as f32
        } else if stream.r_frame_rate().denominator() != 0 {
            stream.r_frame_rate().numerator() as f32 / stream.r_frame_rate().denominator() as f32
        } else {
            24.0 // Default fallback
        };
        
        let bitrate = if stream.bit_rate() > 0 {
            Some(stream.bit_rate() as u32)
        } else {
            None
        };
        
        let pixel_format = codec_params.format().descriptor()
            .map(|d| d.name().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        
        // Detect color space and HDR
        let (color_space, hdr_metadata) = self.detect_color_space_and_hdr(&stream, codec_params);
        
        Some(VideoStreamInfo {
            index,
            codec,
            width,
            height,
            fps,
            bitrate,
            pixel_format,
            color_space,
            hdr_metadata,
        })
    }
    
    /// Extract audio stream information
    fn extract_audio_stream_info(&self, stream: ffmpeg::format::stream::Stream) -> Option<AudioStreamInfo> {
        let params = stream.parameters();
        let codec_params = params.as_audio().ok()?;
        
        let index = stream.index();
        let codec = ffmpeg::codec::decoder::find(params.id())
            .map(|c| c.name().to_string())
            .unwrap_or_else(|| format!("unknown ({})", params.id().name()));
        
        let sample_rate = codec_params.rate();
        let channels = codec_params.channels() as u32;
        let channel_layout = self.get_channel_layout_name(codec_params.channel_layout());
        
        let bitrate = if stream.bit_rate() > 0 {
            Some(stream.bit_rate() as u32)
        } else {
            None
        };
        
        let sample_format = codec_params.format().name().to_string();
        
        // Extract language from metadata
        let language = stream.metadata()
            .get("language")
            .map(|s| s.to_string());
        
        Some(AudioStreamInfo {
            index,
            codec,
            sample_rate,
            channels,
            channel_layout,
            bitrate,
            sample_format,
            language,
        })
    }
    
    /// Extract subtitle stream information
    fn extract_subtitle_stream_info(&self, stream: ffmpeg::format::stream::Stream) -> Option<SubtitleStreamInfo> {
        let params = stream.parameters();
        
        let index = stream.index();
        let codec = ffmpeg::codec::decoder::find(params.id())
            .map(|c| c.name().to_string())
            .unwrap_or_else(|| format!("unknown ({})", params.id().name()));
        
        let metadata = stream.metadata();
        let language = metadata.get("language").map(|s| s.to_string());
        let title = metadata.get("title").map(|s| s.to_string());
        let forced = metadata.get("forced")
            .map(|s| s == "1" || s.to_lowercase() == "true")
            .unwrap_or(false);
        
        Some(SubtitleStreamInfo {
            index,
            codec,
            language,
            title,
            forced,
        })
    }
    
    /// Detect color space and HDR metadata
    fn detect_color_space_and_hdr(
        &self,
        stream: &ffmpeg::format::stream::Stream,
        video_params: ffmpeg::codec::Parameters,
    ) -> (ColorSpace, Option<HdrMetadata>) {
        let metadata = stream.metadata();
        
        // Check for HDR metadata in stream
        let has_hdr10 = metadata.get("mastering_display_metadata").is_some() ||
                        metadata.get("content_light_level").is_some();
        
        // Check color primaries, transfer characteristics, and matrix
        let color_primaries = video_params.color_primaries();
        let color_transfer = video_params.color_transfer_characteristic();
        let color_space_type = video_params.color_space();
        
        // Determine color space based on metadata
        let color_space = if has_hdr10 {
            ColorSpace::Hdr10
        } else if metadata.get("dolby_vision").is_some() {
            ColorSpace::DolbyVision
        } else if color_primaries == ffmpeg::color::Primaries::BT2020 {
            if color_transfer == ffmpeg::color::TransferCharacteristic::SMPTE2084 {
                ColorSpace::Hdr10
            } else if color_transfer == ffmpeg::color::TransferCharacteristic::ARIB_STD_B67 {
                ColorSpace::Hlg
            } else {
                ColorSpace::Bt2020
            }
        } else if color_primaries == ffmpeg::color::Primaries::SMPTE432 {
            ColorSpace::DciP3
        } else {
            ColorSpace::Sdr
        };
        
        // Extract HDR metadata if present
        let hdr_metadata = if has_hdr10 {
            self.extract_hdr_metadata(metadata)
        } else {
            None
        };
        
        (color_space, hdr_metadata)
    }
    
    /// Extract HDR metadata from stream metadata
    fn extract_hdr_metadata(&self, metadata: &ffmpeg::Dictionary) -> Option<HdrMetadata> {
        // Parse content light level
        let (max_cll, max_fall) = if let Some(cll) = metadata.get("content_light_level") {
            // Format: "max_cll,max_fall"
            let parts: Vec<&str> = cll.split(',').collect();
            if parts.len() == 2 {
                (
                    parts[0].parse::<u32>().unwrap_or(0),
                    parts[1].parse::<u32>().unwrap_or(0),
                )
            } else {
                (0, 0)
            }
        } else {
            (0, 0)
        };
        
        // Parse mastering display metadata
        let mastering_display = if let Some(mdm) = metadata.get("mastering_display_metadata") {
            self.parse_mastering_display_metadata(mdm)
        } else {
            None
        };
        
        Some(HdrMetadata {
            max_cll,
            max_fall,
            mastering_display,
        })
    }
    
    /// Parse mastering display metadata string
    fn parse_mastering_display_metadata(&self, mdm: &str) -> Option<MasteringDisplay> {
        // Format: "G(x,y)B(x,y)R(x,y)WP(x,y)L(max,min)"
        // Example: "G(13250,34500)B(7500,3000)R(34000,16000)WP(15635,16450)L(10000000,50)"
        
        let mut values = HashMap::new();
        
        // Parse color components
        for component in mdm.split(')') {
            if let Some(idx) = component.find('(') {
                let name = &component[..idx];
                let coords = &component[idx + 1..];
                
                if let Some(comma_idx) = coords.find(',') {
                    let x = coords[..comma_idx].parse::<f32>().ok();
                    let y = coords[comma_idx + 1..].parse::<f32>().ok();
                    
                    if let (Some(x), Some(y)) = (x, y) {
                        values.insert(name, (x, y));
                    }
                }
            }
        }
        
        // Extract values (chromaticity values are typically in 0.00002 units)
        let scale = 50000.0;
        
        Some(MasteringDisplay {
            red_x: values.get("R").map(|(x, _)| x / scale).unwrap_or(0.64),
            red_y: values.get("R").map(|(_, y)| y / scale).unwrap_or(0.33),
            green_x: values.get("G").map(|(x, _)| x / scale).unwrap_or(0.30),
            green_y: values.get("G").map(|(_, y)| y / scale).unwrap_or(0.60),
            blue_x: values.get("B").map(|(x, _)| x / scale).unwrap_or(0.15),
            blue_y: values.get("B").map(|(_, y)| y / scale).unwrap_or(0.06),
            white_x: values.get("WP").map(|(x, _)| x / scale).unwrap_or(0.3127),
            white_y: values.get("WP").map(|(_, y)| y / scale).unwrap_or(0.3290),
            max_luminance: values.get("L").map(|(x, _)| x / 10000.0).unwrap_or(1000.0),
            min_luminance: values.get("L").map(|(_, y)| y / 10000.0).unwrap_or(0.005),
        })
    }
    
    /// Get channel layout name
    fn get_channel_layout_name(&self, layout: ffmpeg::channel_layout::ChannelLayout) -> String {
        match layout {
            ffmpeg::channel_layout::ChannelLayout::MONO => "mono".to_string(),
            ffmpeg::channel_layout::ChannelLayout::STEREO => "stereo".to_string(),
            ffmpeg::channel_layout::ChannelLayout::_2POINT1 => "2.1".to_string(),
            ffmpeg::channel_layout::ChannelLayout::_3POINT1 => "3.1".to_string(),
            ffmpeg::channel_layout::ChannelLayout::_4POINT0 => "4.0".to_string(),
            ffmpeg::channel_layout::ChannelLayout::_4POINT1 => "4.1".to_string(),
            ffmpeg::channel_layout::ChannelLayout::_5POINT0 => "5.0".to_string(),
            ffmpeg::channel_layout::ChannelLayout::_5POINT1 => "5.1".to_string(),
            ffmpeg::channel_layout::ChannelLayout::_7POINT1 => "7.1".to_string(),
            _ => format!("{} channels", layout.channels()),
        }
    }
}

/// Metadata parser for extracting media metadata
struct MetadataParser {
    /// Known metadata keys to extract
    known_keys: Vec<&'static str>,
}

impl MetadataParser {
    fn new() -> Self {
        Self {
            known_keys: vec![
                "title", "artist", "album", "date", "year", "genre", 
                "comment", "track", "albumartist", "composer", "copyright",
                "description", "encoder", "language",
            ],
        }
    }
    
    /// Parse metadata from FFmpeg dictionary
    fn parse_metadata(&self, dict: &ffmpeg::Dictionary) -> MediaMetadata {
        let mut metadata = MediaMetadata::default();
        let mut custom = HashMap::new();
        
        for (key, value) in dict.iter() {
            match key.to_lowercase().as_str() {
                "title" => metadata.title = Some(value.to_string()),
                "artist" | "albumartist" => metadata.artist = Some(value.to_string()),
                "album" => metadata.album = Some(value.to_string()),
                "date" | "year" => {
                    if let Ok(year) = value.parse::<u32>() {
                        metadata.year = Some(year);
                    } else if value.len() >= 4 {
                        // Try to extract year from date string
                        if let Ok(year) = value[..4].parse::<u32>() {
                            metadata.year = Some(year);
                        }
                    }
                }
                "genre" => metadata.genre = Some(value.to_string()),
                "comment" => metadata.comment = Some(value.to_string()),
                "track" => {
                    // Handle "track/total" format
                    if let Some(slash_idx) = value.find('/') {
                        if let Ok(track) = value[..slash_idx].parse::<u32>() {
                            metadata.track = Some(track);
                        }
                    } else if let Ok(track) = value.parse::<u32>() {
                        metadata.track = Some(track);
                    }
                }
                _ => {
                    // Store other metadata as custom
                    custom.insert(key.to_string(), value.to_string());
                }
            }
        }
        
        metadata.custom = custom;
        metadata
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_metadata_parser() {
        let parser = MetadataParser::new();
        assert!(!parser.known_keys.is_empty());
    }
    
    #[test]
    fn test_stream_info_extractor_creation() {
        let extractor = StreamInfoExtractor::new();
        // Just verify it can be created
        let _ = extractor;
    }
}