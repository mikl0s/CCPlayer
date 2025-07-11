//! Audio output module for CCPlayer
//! 
//! This module handles audio playback using cpal for cross-platform
//! audio output. It manages audio synchronization, volume control,
//! and audio device selection.

mod cpal_output;
mod device;
mod sync;
mod volume;

pub use cpal_output::CpalAudioOutput;
pub use device::{DeviceManager, DeviceChangeListener, DeviceCapabilities};
pub use sync::{AVSyncController, SyncMode, FrameAction, MasterClock, VideoClock, SyncStats};
pub use volume::{VolumeController, RampType};

use crate::utils::error::Result;
use crate::decoder::AudioSamples;
use std::sync::Arc;

/// Audio output trait defining the interface for audio playback
pub trait AudioOutput: Send + Sync {
    /// Create a new audio output instance
    /// 
    /// # Returns
    /// 
    /// Returns the audio output instance or an error
    fn new() -> Result<Self> where Self: Sized;
    
    /// Initialize audio output with specific format
    /// 
    /// # Arguments
    /// 
    /// * `format` - Audio format specification
    fn initialize(&mut self, format: AudioFormat) -> Result<()>;
    
    /// Play audio samples
    /// 
    /// # Arguments
    /// 
    /// * `samples` - Audio samples to play
    fn play(&mut self, samples: &AudioSamples) -> Result<()>;
    
    /// Pause audio playback
    fn pause(&mut self) -> Result<()>;
    
    /// Resume audio playback
    fn resume(&mut self) -> Result<()>;
    
    /// Stop audio playback and clear buffers
    fn stop(&mut self) -> Result<()>;
    
    /// Set volume level
    /// 
    /// # Arguments
    /// 
    /// * `volume` - Volume level (0.0 to 1.0)
    fn set_volume(&mut self, volume: f32) -> Result<()>;
    
    /// Get current volume level
    fn get_volume(&self) -> f32;
    
    /// Get current playback position
    /// 
    /// # Returns
    /// 
    /// Current playback timestamp in microseconds
    fn get_position(&self) -> i64;
    
    /// Get audio latency
    /// 
    /// # Returns
    /// 
    /// Audio latency in microseconds
    fn get_latency(&self) -> i64;
    
    /// Check if audio is playing
    fn is_playing(&self) -> bool;
    
    /// Get buffer status
    /// 
    /// # Returns
    /// 
    /// Buffer fill level (0.0 to 1.0)
    fn get_buffer_fill(&self) -> f32;
    
    /// Set audio device
    /// 
    /// # Arguments
    /// 
    /// * `device` - Audio device to use
    fn set_device(&mut self, device: AudioDevice) -> Result<()>;
    
    /// Get available audio devices
    fn get_devices() -> Result<Vec<AudioDevice>>;
}

/// Audio format specification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioFormat {
    /// Sample rate in Hz
    pub sample_rate: u32,
    
    /// Number of channels
    pub channels: u16,
    
    /// Sample format
    pub sample_format: SampleFormat,
    
    /// Channel layout
    pub channel_layout: ChannelLayout,
}

impl Default for AudioFormat {
    fn default() -> Self {
        Self {
            sample_rate: 48000,
            channels: 2,
            sample_format: SampleFormat::F32,
            channel_layout: ChannelLayout::Stereo,
        }
    }
}

/// Audio sample format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleFormat {
    /// 8-bit unsigned integer
    U8,
    
    /// 16-bit signed integer
    I16,
    
    /// 24-bit signed integer
    I24,
    
    /// 32-bit signed integer
    I32,
    
    /// 32-bit floating point
    F32,
    
    /// 64-bit floating point
    F64,
}

/// Channel layout
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelLayout {
    /// Mono (1 channel)
    Mono,
    
    /// Stereo (2 channels)
    Stereo,
    
    /// 2.1 (3 channels)
    Surround21,
    
    /// 5.1 (6 channels)
    Surround51,
    
    /// 7.1 (8 channels)
    Surround71,
    
    /// Custom channel count
    Custom(u16),
}

impl ChannelLayout {
    /// Get the number of channels for this layout
    pub fn channel_count(&self) -> u16 {
        match self {
            Self::Mono => 1,
            Self::Stereo => 2,
            Self::Surround21 => 3,
            Self::Surround51 => 6,
            Self::Surround71 => 8,
            Self::Custom(count) => *count,
        }
    }
}

/// Audio device information
#[derive(Debug, Clone)]
pub struct AudioDevice {
    /// Device name
    pub name: String,
    
    /// Device ID
    pub id: String,
    
    /// Whether this is the default device
    pub is_default: bool,
    
    /// Supported sample rates
    pub sample_rates: Vec<u32>,
    
    /// Maximum channels
    pub max_channels: u16,
    
    /// Device type
    pub device_type: AudioDeviceType,
}

/// Audio device type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioDeviceType {
    /// Built-in speakers
    Speakers,
    
    /// Headphones
    Headphones,
    
    /// HDMI output
    Hdmi,
    
    /// Digital output (S/PDIF, etc.)
    Digital,
    
    /// Bluetooth device
    Bluetooth,
    
    /// USB audio device
    Usb,
    
    /// Virtual device
    Virtual,
    
    /// Unknown type
    Unknown,
}

/// Audio processing options
#[derive(Debug, Clone)]
pub struct AudioProcessingOptions {
    /// Enable volume normalization
    pub normalize: bool,
    
    /// Target normalization level (LUFS)
    pub normalization_target: f32,
    
    /// Enable dynamic range compression
    pub compress: bool,
    
    /// Compression ratio
    pub compression_ratio: f32,
    
    /// Enable equalizer
    pub equalizer: bool,
    
    /// Equalizer bands (frequency, gain)
    pub eq_bands: Vec<(f32, f32)>,
    
    /// Audio delay in milliseconds (for sync adjustment)
    pub delay_ms: i32,
}

impl Default for AudioProcessingOptions {
    fn default() -> Self {
        Self {
            normalize: false,
            normalization_target: -14.0, // Standard for streaming
            compress: false,
            compression_ratio: 2.0,
            equalizer: false,
            eq_bands: vec![],
            delay_ms: 0,
        }
    }
}

/// Audio statistics for monitoring
#[derive(Debug, Clone, Copy, Default)]
pub struct AudioStats {
    /// Current buffer underruns
    pub underruns: u64,
    
    /// Total samples played
    pub samples_played: u64,
    
    /// Current sample rate
    pub sample_rate: u32,
    
    /// Current bit depth
    pub bit_depth: u32,
    
    /// Peak level (0.0 to 1.0)
    pub peak_level: f32,
    
    /// RMS level (0.0 to 1.0)
    pub rms_level: f32,
}

/// Audio event callbacks
pub trait AudioEventHandler: Send + Sync {
    /// Called when audio device is changed
    fn on_device_change(&mut self, device: &AudioDevice);
    
    /// Called when audio format changes
    fn on_format_change(&mut self, format: &AudioFormat);
    
    /// Called on buffer underrun
    fn on_underrun(&mut self);
    
    /// Called when playback ends
    fn on_playback_end(&mut self);
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_audio_format_default() {
        let format = AudioFormat::default();
        assert_eq!(format.sample_rate, 48000);
        assert_eq!(format.channels, 2);
        assert_eq!(format.sample_format, SampleFormat::F32);
        assert_eq!(format.channel_layout, ChannelLayout::Stereo);
    }
    
    #[test]
    fn test_channel_layout_count() {
        assert_eq!(ChannelLayout::Mono.channel_count(), 1);
        assert_eq!(ChannelLayout::Stereo.channel_count(), 2);
        assert_eq!(ChannelLayout::Surround51.channel_count(), 6);
        assert_eq!(ChannelLayout::Surround71.channel_count(), 8);
        assert_eq!(ChannelLayout::Custom(10).channel_count(), 10);
    }
    
    #[test]
    fn test_audio_processing_options_default() {
        let options = AudioProcessingOptions::default();
        assert!(!options.normalize);
        assert_eq!(options.normalization_target, -14.0);
        assert!(!options.compress);
        assert_eq!(options.compression_ratio, 2.0);
        assert!(!options.equalizer);
        assert!(options.eq_bands.is_empty());
        assert_eq!(options.delay_ms, 0);
    }
}