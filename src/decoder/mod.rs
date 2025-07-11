//! Decoder module for CCPlayer
//! 
//! This module handles video and audio decoding using FFmpeg through
//! rusty_ffmpeg bindings. It supports hardware acceleration and various
//! video codecs.

mod ffmpeg_decoder;
mod frame_queue;
mod hw_accel;
mod stream_info;

pub use ffmpeg_decoder::FFmpegDecoder;
pub use frame_queue::{FrameQueue, FrameTimingController, FramePresentation};
pub use hw_accel::{HardwareAccelerator, HwAccelConfig};
pub use stream_info::StreamInfoExtractor;

use crate::utils::error::Result;
use crate::renderer::VideoFrame;
use std::path::Path;
use std::time::Duration;

/// Decoder trait defining the interface for media decoding
pub trait Decoder: Send + Sync {
    /// Create a new decoder instance
    /// 
    /// # Returns
    /// 
    /// Returns the decoder instance or an error
    fn new() -> Result<Self> where Self: Sized;
    
    /// Open a media file for decoding
    /// 
    /// # Arguments
    /// 
    /// * `path` - Path to the media file
    /// 
    /// # Returns
    /// 
    /// Returns media information or an error
    fn open_file(&mut self, path: &Path) -> Result<MediaInfo>;
    
    /// Open a media stream from URL
    /// 
    /// # Arguments
    /// 
    /// * `url` - URL of the media stream
    /// 
    /// # Returns
    /// 
    /// Returns media information or an error
    fn open_url(&mut self, url: &str) -> Result<MediaInfo>;
    
    /// Decode the next video frame
    /// 
    /// # Returns
    /// 
    /// Returns the decoded frame or None if no more frames
    fn decode_frame(&mut self) -> Result<Option<VideoFrame>>;
    
    /// Decode the next audio samples
    /// 
    /// # Returns
    /// 
    /// Returns audio samples or None if no more samples
    fn decode_audio(&mut self) -> Result<Option<AudioSamples>>;
    
    /// Seek to a specific timestamp
    /// 
    /// # Arguments
    /// 
    /// * `timestamp` - Target timestamp
    fn seek(&mut self, timestamp: Duration) -> Result<()>;
    
    /// Get current playback position
    /// 
    /// # Returns
    /// 
    /// Current timestamp
    fn position(&self) -> Duration;
    
    /// Check if end of stream is reached
    fn is_eof(&self) -> bool;
    
    /// Flush decoder buffers
    fn flush(&mut self) -> Result<()>;
    
    /// Enable or disable hardware acceleration
    /// 
    /// # Arguments
    /// 
    /// * `enabled` - Whether to enable hardware acceleration
    fn set_hardware_acceleration(&mut self, enabled: bool) -> Result<()>;
}

/// Media information
#[derive(Debug, Clone)]
pub struct MediaInfo {
    /// File path or URL
    pub source: String,
    
    /// Total duration
    pub duration: Duration,
    
    /// Video streams
    pub video_streams: Vec<VideoStreamInfo>,
    
    /// Audio streams
    pub audio_streams: Vec<AudioStreamInfo>,
    
    /// Subtitle streams
    pub subtitle_streams: Vec<SubtitleStreamInfo>,
    
    /// Container format
    pub format: String,
    
    /// File size in bytes (if available)
    pub file_size: Option<u64>,
    
    /// Bitrate in bits per second
    pub bitrate: Option<u32>,
    
    /// Metadata tags
    pub metadata: MediaMetadata,
}

/// Video stream information
#[derive(Debug, Clone)]
pub struct VideoStreamInfo {
    /// Stream index
    pub index: usize,
    
    /// Codec name
    pub codec: String,
    
    /// Video width
    pub width: u32,
    
    /// Video height
    pub height: u32,
    
    /// Frame rate (frames per second)
    pub fps: f32,
    
    /// Bitrate in bits per second
    pub bitrate: Option<u32>,
    
    /// Pixel format
    pub pixel_format: String,
    
    /// Color space
    pub color_space: ColorSpace,
    
    /// HDR metadata if available
    pub hdr_metadata: Option<HdrMetadata>,
}

/// Audio stream information
#[derive(Debug, Clone)]
pub struct AudioStreamInfo {
    /// Stream index
    pub index: usize,
    
    /// Codec name
    pub codec: String,
    
    /// Sample rate in Hz
    pub sample_rate: u32,
    
    /// Number of channels
    pub channels: u32,
    
    /// Channel layout
    pub channel_layout: String,
    
    /// Bitrate in bits per second
    pub bitrate: Option<u32>,
    
    /// Sample format
    pub sample_format: String,
    
    /// Language tag
    pub language: Option<String>,
}

/// Subtitle stream information
#[derive(Debug, Clone)]
pub struct SubtitleStreamInfo {
    /// Stream index
    pub index: usize,
    
    /// Codec name
    pub codec: String,
    
    /// Language tag
    pub language: Option<String>,
    
    /// Title
    pub title: Option<String>,
    
    /// Whether this is a forced subtitle
    pub forced: bool,
}

/// Media metadata
#[derive(Debug, Clone, Default)]
pub struct MediaMetadata {
    /// Title
    pub title: Option<String>,
    
    /// Artist/Author
    pub artist: Option<String>,
    
    /// Album
    pub album: Option<String>,
    
    /// Year
    pub year: Option<u32>,
    
    /// Genre
    pub genre: Option<String>,
    
    /// Comment
    pub comment: Option<String>,
    
    /// Track number
    pub track: Option<u32>,
    
    /// Custom tags
    pub custom: std::collections::HashMap<String, String>,
}

/// Color space information
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSpace {
    /// Standard dynamic range BT.709
    Sdr,
    
    /// BT.2020 color space
    Bt2020,
    
    /// DCI-P3 color space
    DciP3,
    
    /// HDR10
    Hdr10,
    
    /// HDR10+
    Hdr10Plus,
    
    /// Dolby Vision
    DolbyVision,
    
    /// Hybrid Log-Gamma
    Hlg,
}

/// HDR metadata
#[derive(Debug, Clone)]
pub struct HdrMetadata {
    /// Maximum content light level
    pub max_cll: u32,
    
    /// Maximum frame average light level
    pub max_fall: u32,
    
    /// Mastering display metadata
    pub mastering_display: Option<MasteringDisplay>,
}

/// Mastering display metadata
#[derive(Debug, Clone)]
pub struct MasteringDisplay {
    /// Red primary chromaticity
    pub red_x: f32,
    pub red_y: f32,
    
    /// Green primary chromaticity
    pub green_x: f32,
    pub green_y: f32,
    
    /// Blue primary chromaticity
    pub blue_x: f32,
    pub blue_y: f32,
    
    /// White point chromaticity
    pub white_x: f32,
    pub white_y: f32,
    
    /// Maximum luminance in nits
    pub max_luminance: f32,
    
    /// Minimum luminance in nits
    pub min_luminance: f32,
}

/// Audio samples from decoder
#[derive(Debug, Clone)]
pub struct AudioSamples {
    /// Sample data (interleaved if multi-channel)
    pub data: Vec<f32>,
    
    /// Number of samples per channel
    pub sample_count: usize,
    
    /// Number of channels
    pub channels: usize,
    
    /// Sample rate
    pub sample_rate: u32,
    
    /// Presentation timestamp
    pub pts: i64,
}

/// Decoder capabilities
#[derive(Debug, Clone)]
pub struct DecoderCapabilities {
    /// Supported video codecs
    pub video_codecs: Vec<CodecInfo>,
    
    /// Supported audio codecs
    pub audio_codecs: Vec<CodecInfo>,
    
    /// Available hardware acceleration methods
    pub hw_accel_methods: Vec<HwAccelMethod>,
}

/// Codec information
#[derive(Debug, Clone)]
pub struct CodecInfo {
    /// Codec name
    pub name: String,
    
    /// Codec long name
    pub long_name: String,
    
    /// Whether hardware acceleration is available
    pub hw_accel_available: bool,
}

/// Hardware acceleration method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HwAccelMethod {
    /// No hardware acceleration
    None,
    
    /// NVIDIA NVDEC/NVENC
    Nvdec,
    
    /// Intel Quick Sync Video
    Qsv,
    
    /// AMD VCE/VCN
    Amf,
    
    /// Video Acceleration API (Linux)
    Vaapi,
    
    /// Video Decode Acceleration (macOS)
    Vda,
    
    /// VideoToolbox (macOS)
    VideoToolbox,
    
    /// Direct3D11 Video Acceleration (Windows)
    D3d11va,
    
    /// DirectX Video Acceleration 2 (Windows)
    Dxva2,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_color_space() {
        assert_ne!(ColorSpace::Sdr, ColorSpace::Hdr10);
        assert_eq!(ColorSpace::Hdr10, ColorSpace::Hdr10);
    }
    
    #[test]
    fn test_media_metadata_default() {
        let metadata = MediaMetadata::default();
        assert!(metadata.title.is_none());
        assert!(metadata.artist.is_none());
        assert!(metadata.custom.is_empty());
    }
    
    #[test]
    fn test_hw_accel_method() {
        assert_ne!(HwAccelMethod::None, HwAccelMethod::Nvdec);
        assert_eq!(HwAccelMethod::Nvdec, HwAccelMethod::Nvdec);
    }
}