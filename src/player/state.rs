//! Player state management for CCPlayer
//!
//! This module provides state management, configuration, and statistics
//! tracking for the media player.

use crate::decoder::MediaInfo;
use crate::player::{PlaybackState, PlayerConfig, Playlist, PlaybackStats};
use std::sync::{Arc, RwLock, Mutex};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use log::{info, debug};

/// Player state manager
pub struct PlayerStateManager {
    /// Current state
    state: Arc<RwLock<PlayerStateData>>,
    
    /// Configuration
    config: Arc<RwLock<PlayerConfig>>,
    
    /// Statistics tracker
    stats_tracker: Arc<Mutex<StatsTracker>>,
    
    /// Position history for resuming
    position_history: Arc<Mutex<PositionHistory>>,
}

/// Internal player state data
#[derive(Debug, Clone)]
pub struct PlayerStateData {
    /// Current playback state
    pub playback_state: PlaybackState,
    
    /// Current media info
    pub media_info: Option<MediaInfo>,
    
    /// Current position in microseconds
    pub position_us: i64,
    
    /// Duration in microseconds
    pub duration_us: i64,
    
    /// Playback speed
    pub speed: f32,
    
    /// Volume level (0.0 to 1.0)
    pub volume: f32,
    
    /// Previous volume (for mute/unmute)
    pub previous_volume: f32,
    
    /// Muted state
    pub muted: bool,
    
    /// Fullscreen state
    pub fullscreen: bool,
    
    /// Current playlist
    pub playlist: Playlist,
    
    /// Buffering percentage
    pub buffer_percent: f32,
    
    /// Last error message
    pub last_error: Option<String>,
}

impl Default for PlayerStateData {
    fn default() -> Self {
        Self {
            playback_state: PlaybackState::Idle,
            media_info: None,
            position_us: 0,
            duration_us: 0,
            speed: 1.0,
            volume: 0.7,
            previous_volume: 0.7,
            muted: false,
            fullscreen: false,
            playlist: Playlist {
                items: Vec::new(),
                current_index: None,
                shuffle: false,
                repeat_mode: crate::player::RepeatMode::None,
            },
            buffer_percent: 0.0,
            last_error: None,
        }
    }
}

/// Statistics tracker
#[derive(Debug)]
struct StatsTracker {
    /// Start time
    start_time: Option<Instant>,
    
    /// Total playback time
    total_playback_time: Duration,
    
    /// Frame statistics
    frames_rendered: u64,
    frames_dropped: u64,
    
    /// Audio statistics
    audio_samples_played: u64,
    audio_underruns: u64,
    
    /// Bitrate measurements
    video_bitrate_samples: Vec<(Instant, u32)>,
    audio_bitrate_samples: Vec<(Instant, u32)>,
    
    /// Performance metrics
    cpu_samples: Vec<(Instant, f32)>,
    memory_samples: Vec<(Instant, f32)>,
}

impl StatsTracker {
    fn new() -> Self {
        Self {
            start_time: None,
            total_playback_time: Duration::ZERO,
            frames_rendered: 0,
            frames_dropped: 0,
            audio_samples_played: 0,
            audio_underruns: 0,
            video_bitrate_samples: Vec::with_capacity(60),
            audio_bitrate_samples: Vec::with_capacity(60),
            cpu_samples: Vec::with_capacity(60),
            memory_samples: Vec::with_capacity(60),
        }
    }
    
    fn start_playback(&mut self) {
        self.start_time = Some(Instant::now());
    }
    
    fn stop_playback(&mut self) {
        if let Some(start) = self.start_time.take() {
            self.total_playback_time += start.elapsed();
        }
    }
    
    fn update_frame_stats(&mut self, rendered: u64, dropped: u64) {
        self.frames_rendered = rendered;
        self.frames_dropped = dropped;
    }
    
    fn add_bitrate_sample(&mut self, video: u32, audio: u32) {
        let now = Instant::now();
        
        // Keep only last 60 seconds of samples
        let cutoff = now - Duration::from_secs(60);
        
        self.video_bitrate_samples.retain(|(t, _)| *t > cutoff);
        self.audio_bitrate_samples.retain(|(t, _)| *t > cutoff);
        
        self.video_bitrate_samples.push((now, video));
        self.audio_bitrate_samples.push((now, audio));
    }
    
    fn get_average_bitrates(&self) -> (u32, u32) {
        let video_avg = if self.video_bitrate_samples.is_empty() {
            0
        } else {
            let sum: u32 = self.video_bitrate_samples.iter().map(|(_, b)| b).sum();
            sum / self.video_bitrate_samples.len() as u32
        };
        
        let audio_avg = if self.audio_bitrate_samples.is_empty() {
            0
        } else {
            let sum: u32 = self.audio_bitrate_samples.iter().map(|(_, b)| b).sum();
            sum / self.audio_bitrate_samples.len() as u32
        };
        
        (video_avg, audio_avg)
    }
    
    fn to_playback_stats(&self) -> PlaybackStats {
        let (video_bitrate, audio_bitrate) = self.get_average_bitrates();
        
        PlaybackStats {
            frames_rendered: self.frames_rendered,
            frames_dropped: self.frames_dropped,
            audio_samples_played: self.audio_samples_played,
            video_bitrate,
            audio_bitrate,
            buffer_health: 1.0, // TODO: Calculate from actual buffer state
            cpu_usage: self.cpu_samples.last().map(|(_, v)| *v).unwrap_or(0.0),
            memory_usage: self.memory_samples.last().map(|(_, v)| *v).unwrap_or(0.0),
        }
    }
}

/// Position history for resume functionality
#[derive(Debug, Serialize, Deserialize)]
struct PositionHistory {
    /// Map of file path to last position
    positions: HashMap<String, PositionEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PositionEntry {
    /// Last position in microseconds
    position_us: i64,
    
    /// Total duration in microseconds
    duration_us: i64,
    
    /// Last played timestamp
    last_played: u64, // Unix timestamp
}

impl PositionHistory {
    fn new() -> Self {
        Self {
            positions: HashMap::new(),
        }
    }
    
    fn save_position(&mut self, path: &str, position_us: i64, duration_us: i64) {
        // Don't save if near the beginning or end
        let position_percent = position_us as f64 / duration_us as f64;
        if position_percent < 0.05 || position_percent > 0.95 {
            self.positions.remove(path);
            return;
        }
        
        let entry = PositionEntry {
            position_us,
            duration_us,
            last_played: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        
        self.positions.insert(path.to_string(), entry);
        self.save_to_disk();
    }
    
    fn get_position(&self, path: &str) -> Option<i64> {
        self.positions.get(path).map(|e| e.position_us)
    }
    
    fn load_from_disk(&mut self) {
        if let Ok(data) = std::fs::read_to_string(Self::history_file_path()) {
            if let Ok(history) = serde_json::from_str::<PositionHistory>(&data) {
                self.positions = history.positions;
            }
        }
    }
    
    fn save_to_disk(&self) {
        if let Ok(data) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(Self::history_file_path(), data);
        }
    }
    
    fn history_file_path() -> std::path::PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
        path.push("ccplayer");
        let _ = std::fs::create_dir_all(&path);
        path.push("position_history.json");
        path
    }
}

impl PlayerStateManager {
    /// Create a new state manager
    pub fn new(config: PlayerConfig) -> Self {
        let mut position_history = PositionHistory::new();
        position_history.load_from_disk();
        
        Self {
            state: Arc::new(RwLock::new(PlayerStateData::default())),
            config: Arc::new(RwLock::new(config)),
            stats_tracker: Arc::new(Mutex::new(StatsTracker::new())),
            position_history: Arc::new(Mutex::new(position_history)),
        }
    }
    
    /// Get current state
    pub fn get_state(&self) -> PlayerStateData {
        self.state.read().unwrap().clone()
    }
    
    /// Update playback state
    pub fn set_playback_state(&self, state: PlaybackState) {
        let mut data = self.state.write().unwrap();
        data.playback_state = state;
        
        // Update stats tracker
        match state {
            PlaybackState::Playing => {
                self.stats_tracker.lock().unwrap().start_playback();
            }
            PlaybackState::Paused | PlaybackState::Stopped => {
                self.stats_tracker.lock().unwrap().stop_playback();
            }
            _ => {}
        }
        
        info!("Playback state changed to: {:?}", state);
    }
    
    /// Set media info
    pub fn set_media_info(&self, info: MediaInfo) {
        let mut data = self.state.write().unwrap();
        data.duration_us = info.duration.as_micros() as i64;
        
        // Check for saved position
        if self.config.read().unwrap().remember_position {
            if let Some(position) = self.position_history.lock().unwrap().get_position(&info.source) {
                data.position_us = position;
                info!("Restored position: {:?}", Duration::from_micros(position as u64));
            }
        }
        
        data.media_info = Some(info);
    }
    
    /// Update position
    pub fn update_position(&self, position_us: i64) {
        let mut data = self.state.write().unwrap();
        data.position_us = position_us;
        
        // Save position periodically
        if self.config.read().unwrap().remember_position {
            if let Some(info) = &data.media_info {
                if position_us % 5_000_000 < 100_000 { // Every ~5 seconds
                    self.position_history.lock().unwrap().save_position(
                        &info.source,
                        position_us,
                        data.duration_us,
                    );
                }
            }
        }
    }
    
    /// Set volume
    pub fn set_volume(&self, volume: f32) {
        let mut data = self.state.write().unwrap();
        if !data.muted {
            data.previous_volume = data.volume;
        }
        data.volume = volume.clamp(0.0, 1.0);
        debug!("Volume set to: {:.2}", data.volume);
    }
    
    /// Toggle mute
    pub fn toggle_mute(&self) -> f32 {
        let mut data = self.state.write().unwrap();
        data.muted = !data.muted;
        
        if data.muted {
            data.previous_volume = data.volume;
            data.volume = 0.0;
        } else {
            data.volume = data.previous_volume;
        }
        
        data.volume
    }
    
    /// Set playback speed
    pub fn set_speed(&self, speed: f32) {
        let mut data = self.state.write().unwrap();
        data.speed = speed.clamp(0.25, 4.0);
        info!("Playback speed set to: {:.2}x", data.speed);
    }
    
    /// Set fullscreen
    pub fn set_fullscreen(&self, fullscreen: bool) {
        let mut data = self.state.write().unwrap();
        data.fullscreen = fullscreen;
    }
    
    /// Update buffer status
    pub fn update_buffer(&self, percent: f32) {
        let mut data = self.state.write().unwrap();
        data.buffer_percent = percent.clamp(0.0, 100.0);
    }
    
    /// Set error
    pub fn set_error(&self, error: Option<String>) {
        let mut data = self.state.write().unwrap();
        data.last_error = error;
        if error.is_some() {
            data.playback_state = PlaybackState::Error;
        }
    }
    
    /// Update statistics
    pub fn update_stats(&self, frames_rendered: u64, frames_dropped: u64) {
        self.stats_tracker.lock().unwrap().update_frame_stats(frames_rendered, frames_dropped);
    }
    
    /// Add bitrate sample
    pub fn add_bitrate_sample(&self, video: u32, audio: u32) {
        self.stats_tracker.lock().unwrap().add_bitrate_sample(video, audio);
    }
    
    /// Get playback statistics
    pub fn get_stats(&self) -> PlaybackStats {
        self.stats_tracker.lock().unwrap().to_playback_stats()
    }
    
    /// Get configuration
    pub fn get_config(&self) -> PlayerConfig {
        self.config.read().unwrap().clone()
    }
    
    /// Update configuration
    pub fn update_config<F>(&self, updater: F)
    where
        F: FnOnce(&mut PlayerConfig),
    {
        let mut config = self.config.write().unwrap();
        updater(&mut *config);
        self.save_config();
    }
    
    /// Load configuration from disk
    pub fn load_config(&self) {
        if let Ok(data) = std::fs::read_to_string(Self::config_file_path()) {
            if let Ok(config) = serde_json::from_str::<PlayerConfig>(&data) {
                *self.config.write().unwrap() = config;
                info!("Loaded configuration from disk");
            }
        }
    }
    
    /// Save configuration to disk
    fn save_config(&self) {
        let config = self.config.read().unwrap();
        if let Ok(data) = serde_json::to_string_pretty(&*config) {
            let _ = std::fs::write(Self::config_file_path(), data);
        }
    }
    
    fn config_file_path() -> std::path::PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
        path.push("ccplayer");
        let _ = std::fs::create_dir_all(&path);
        path.push("config.json");
        path
    }
}

/// State change event
#[derive(Debug, Clone)]
pub enum StateChangeEvent {
    PlaybackStateChanged(PlaybackState),
    MediaLoaded(MediaInfo),
    PositionChanged(Duration),
    VolumeChanged(f32),
    SpeedChanged(f32),
    BufferChanged(f32),
    ErrorOccurred(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_player_state_default() {
        let state = PlayerStateData::default();
        assert_eq!(state.playback_state, PlaybackState::Idle);
        assert_eq!(state.volume, 0.7);
        assert_eq!(state.speed, 1.0);
        assert!(!state.muted);
        assert!(!state.fullscreen);
    }
    
    #[test]
    fn test_stats_tracker() {
        let mut tracker = StatsTracker::new();
        
        tracker.start_playback();
        std::thread::sleep(Duration::from_millis(100));
        tracker.stop_playback();
        
        assert!(tracker.total_playback_time >= Duration::from_millis(100));
        
        tracker.update_frame_stats(1000, 5);
        let stats = tracker.to_playback_stats();
        assert_eq!(stats.frames_rendered, 1000);
        assert_eq!(stats.frames_dropped, 5);
    }
    
    #[test]
    fn test_position_history() {
        let mut history = PositionHistory::new();
        
        // Test saving position
        history.save_position("test.mp4", 60_000_000, 120_000_000);
        assert_eq!(history.get_position("test.mp4"), Some(60_000_000));
        
        // Test not saving near beginning
        history.save_position("test2.mp4", 1_000_000, 120_000_000);
        assert_eq!(history.get_position("test2.mp4"), None);
        
        // Test not saving near end
        history.save_position("test3.mp4", 119_000_000, 120_000_000);
        assert_eq!(history.get_position("test3.mp4"), None);
    }
    
    #[test]
    fn test_state_manager() {
        let config = PlayerConfig::default();
        let manager = PlayerStateManager::new(config);
        
        // Test volume
        manager.set_volume(0.5);
        assert_eq!(manager.get_state().volume, 0.5);
        
        // Test mute
        let volume = manager.toggle_mute();
        assert_eq!(volume, 0.0);
        assert!(manager.get_state().muted);
        
        let volume = manager.toggle_mute();
        assert_eq!(volume, 0.5);
        assert!(!manager.get_state().muted);
        
        // Test speed
        manager.set_speed(2.0);
        assert_eq!(manager.get_state().speed, 2.0);
        
        // Test clamping
        manager.set_speed(5.0);
        assert_eq!(manager.get_state().speed, 4.0);
    }
}