//! Player controller module for CCPlayer
//! 
//! This module orchestrates the entire media playback process, coordinating
//! between the decoder, renderer, audio output, and window components.
//! It handles playback state, A/V synchronization, and user interactions.

mod controller;
mod state;
mod media_player;

pub use controller::PlayerController;
pub use state::{PlayerStateManager, PlayerStateData, StateChangeEvent};
pub use media_player::{MediaPlayer, MediaPlayerBuilder, PerformanceStats, EventSubscription};

use crate::utils::error::Result;
use crate::window::{Window, WindowEvent};
use crate::renderer::Renderer;
use crate::decoder::{Decoder, MediaInfo};
use crate::audio::AudioOutput;
use std::sync::Arc;
use std::path::Path;
use std::time::Duration;

/// Player trait defining the main media player interface
pub trait Player: Send + Sync {
    /// Create a new player instance
    /// 
    /// # Arguments
    /// 
    /// * `window` - Window for display
    /// * `renderer` - Video renderer
    /// * `decoder` - Media decoder
    /// * `audio` - Audio output
    /// 
    /// # Returns
    /// 
    /// Returns the player instance or an error
    fn new(
        window: Arc<dyn Window>,
        renderer: Arc<dyn Renderer>,
        decoder: Arc<dyn Decoder>,
        audio: Arc<dyn AudioOutput>,
    ) -> Result<Self> where Self: Sized;
    
    /// Load a media file
    /// 
    /// # Arguments
    /// 
    /// * `path` - Path to the media file
    /// 
    /// # Returns
    /// 
    /// Returns media information or an error
    fn load_file(&mut self, path: &Path) -> Result<MediaInfo>;
    
    /// Load a media URL
    /// 
    /// # Arguments
    /// 
    /// * `url` - URL of the media stream
    /// 
    /// # Returns
    /// 
    /// Returns media information or an error
    fn load_url(&mut self, url: &str) -> Result<MediaInfo>;
    
    /// Start or resume playback
    fn play(&mut self) -> Result<()>;
    
    /// Pause playback
    fn pause(&mut self) -> Result<()>;
    
    /// Stop playback
    fn stop(&mut self) -> Result<()>;
    
    /// Toggle play/pause
    fn toggle_play(&mut self) -> Result<()>;
    
    /// Seek to a specific position
    /// 
    /// # Arguments
    /// 
    /// * `position` - Target position
    fn seek(&mut self, position: Duration) -> Result<()>;
    
    /// Seek by a relative amount
    /// 
    /// # Arguments
    /// 
    /// * `delta` - Amount to seek (negative for backward)
    fn seek_relative(&mut self, delta: i64) -> Result<()>;
    
    /// Get current playback state
    fn state(&self) -> PlaybackState;
    
    /// Get current position
    fn position(&self) -> Duration;
    
    /// Get media duration
    fn duration(&self) -> Duration;
    
    /// Set playback speed
    /// 
    /// # Arguments
    /// 
    /// * `speed` - Playback speed multiplier (1.0 = normal)
    fn set_speed(&mut self, speed: f32) -> Result<()>;
    
    /// Get current playback speed
    fn speed(&self) -> f32;
    
    /// Set volume
    /// 
    /// # Arguments
    /// 
    /// * `volume` - Volume level (0.0 to 1.0)
    fn set_volume(&mut self, volume: f32) -> Result<()>;
    
    /// Get current volume
    fn volume(&self) -> f32;
    
    /// Mute/unmute audio
    fn toggle_mute(&mut self) -> Result<()>;
    
    /// Check if muted
    fn is_muted(&self) -> bool;
    
    /// Set fullscreen mode
    /// 
    /// # Arguments
    /// 
    /// * `fullscreen` - Whether to enable fullscreen
    fn set_fullscreen(&mut self, fullscreen: bool) -> Result<()>;
    
    /// Check if in fullscreen mode
    fn is_fullscreen(&self) -> bool;
    
    /// Handle window event
    /// 
    /// # Arguments
    /// 
    /// * `event` - Window event to handle
    fn handle_event(&mut self, event: WindowEvent) -> Result<()>;
    
    /// Run the player event loop
    fn run(&mut self) -> Result<()>;
}

/// Playback state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    /// No media loaded
    Idle,
    
    /// Media loaded but not playing
    Stopped,
    
    /// Currently playing
    Playing,
    
    /// Playback paused
    Paused,
    
    /// Buffering media
    Buffering,
    
    /// Seeking to position
    Seeking,
    
    /// End of media reached
    Ended,
    
    /// Error occurred
    Error,
}

/// Player configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlayerConfig {
    /// Auto-play when media is loaded
    pub auto_play: bool,
    
    /// Remember playback position
    pub remember_position: bool,
    
    /// Loop playback
    pub loop_playback: bool,
    
    /// Default volume (0.0 to 1.0)
    pub default_volume: f32,
    
    /// Seek step in seconds
    pub seek_step: u64,
    
    /// Fast seek step in seconds
    pub fast_seek_step: u64,
    
    /// Volume step (0.0 to 1.0)
    pub volume_step: f32,
    
    /// Enable frame dropping for performance
    pub allow_frame_drop: bool,
    
    /// A/V sync threshold in milliseconds
    pub av_sync_threshold: i64,
    
    /// Subtitle settings
    pub subtitle_enabled: bool,
    
    /// Screenshot settings
    pub screenshot_format: ScreenshotFormat,
    pub screenshot_quality: u8,
}

impl Default for PlayerConfig {
    fn default() -> Self {
        Self {
            auto_play: true,
            remember_position: false,
            loop_playback: false,
            default_volume: 0.7,
            seek_step: 10,
            fast_seek_step: 60,
            volume_step: 0.05,
            allow_frame_drop: true,
            av_sync_threshold: 40, // 40ms
            subtitle_enabled: true,
            screenshot_format: ScreenshotFormat::Png,
            screenshot_quality: 90,
        }
    }
}

/// Screenshot format
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ScreenshotFormat {
    Png,
    Jpeg,
    Webp,
}

/// Playback statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct PlaybackStats {
    /// Video frames rendered
    pub frames_rendered: u64,
    
    /// Video frames dropped
    pub frames_dropped: u64,
    
    /// Audio samples played
    pub audio_samples_played: u64,
    
    /// Current video bitrate
    pub video_bitrate: u32,
    
    /// Current audio bitrate
    pub audio_bitrate: u32,
    
    /// Network buffer health (0.0 to 1.0)
    pub buffer_health: f32,
    
    /// CPU usage percentage
    pub cpu_usage: f32,
    
    /// Memory usage in MB
    pub memory_usage: f32,
}

/// Player event for external event handling
#[derive(Debug, Clone)]
pub enum PlayerEvent {
    /// Media loaded
    MediaLoaded { info: MediaInfo },
    
    /// Playback started
    PlaybackStarted,
    
    /// Playback paused
    PlaybackPaused,
    
    /// Playback stopped
    PlaybackStopped,
    
    /// Position changed
    PositionChanged { position: Duration },
    
    /// Buffering progress
    BufferingProgress { percent: f32 },
    
    /// Volume changed
    VolumeChanged { volume: f32 },
    
    /// Playback speed changed
    SpeedChanged { speed: f32 },
    
    /// Error occurred
    Error { message: String },
    
    /// End of media reached
    EndOfMedia,
}

/// Player event handler trait
pub trait PlayerEventHandler: Send + Sync {
    /// Handle player event
    /// 
    /// # Arguments
    /// 
    /// * `event` - Player event
    fn handle_event(&mut self, event: PlayerEvent);
}

/// Playlist management
#[derive(Debug, Clone)]
pub struct Playlist {
    /// List of media items
    pub items: Vec<PlaylistItem>,
    
    /// Current item index
    pub current_index: Option<usize>,
    
    /// Shuffle mode
    pub shuffle: bool,
    
    /// Repeat mode
    pub repeat_mode: RepeatMode,
}

/// Playlist item
#[derive(Debug, Clone)]
pub struct PlaylistItem {
    /// File path or URL
    pub path: String,
    
    /// Display title
    pub title: Option<String>,
    
    /// Duration if known
    pub duration: Option<Duration>,
}

/// Repeat mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepeatMode {
    /// No repeat
    None,
    
    /// Repeat current item
    One,
    
    /// Repeat entire playlist
    All,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_playback_state() {
        assert_ne!(PlaybackState::Idle, PlaybackState::Playing);
        assert_eq!(PlaybackState::Playing, PlaybackState::Playing);
    }
    
    #[test]
    fn test_player_config_default() {
        let config = PlayerConfig::default();
        assert!(config.auto_play);
        assert!(!config.remember_position);
        assert_eq!(config.default_volume, 0.7);
        assert_eq!(config.seek_step, 10);
        assert_eq!(config.volume_step, 0.05);
    }
    
    #[test]
    fn test_repeat_mode() {
        assert_ne!(RepeatMode::None, RepeatMode::One);
        assert_ne!(RepeatMode::One, RepeatMode::All);
    }
}