//! Configuration management for CCPlayer
//! 
//! This module handles loading and managing application configuration
//! from various sources including config files and environment variables.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use crate::utils::error::{CCPlayerError, Result};

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Window configuration
    pub window: WindowConfig,
    
    /// Decoder configuration
    pub decoder: DecoderConfig,
    
    /// Audio configuration
    pub audio: AudioConfig,
    
    /// General application settings
    pub general: GeneralConfig,
}

/// Window configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    /// Initial window width
    pub width: u32,
    
    /// Initial window height
    pub height: u32,
    
    /// Start in fullscreen mode
    pub fullscreen: bool,
    
    /// Window title
    pub title: String,
    
    /// Always on top
    pub always_on_top: bool,
    
    /// Start minimized
    pub start_minimized: bool,
}

/// Decoder configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecoderConfig {
    /// Enable hardware acceleration
    pub hardware_acceleration: bool,
    
    /// Number of threads for software decoding
    pub thread_count: usize,
    
    /// Frame buffer size
    pub buffer_size: usize,
    
    /// Preferred video codec order
    pub preferred_codecs: Vec<String>,
}

/// Audio configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    /// Initial volume (0.0 - 1.0)
    pub volume: f32,
    
    /// Audio buffer size in frames
    pub buffer_size: usize,
    
    /// Sample rate (0 for auto-detect)
    pub sample_rate: u32,
    
    /// Enable audio normalization
    pub normalize: bool,
}

/// General application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// Remember window position and size
    pub remember_window_state: bool,
    
    /// Log level (trace, debug, info, warn, error)
    pub log_level: String,
    
    /// Recent files list size
    pub recent_files_limit: usize,
    
    /// Auto-play on file open
    pub auto_play: bool,
    
    /// Subtitle settings
    pub subtitles: SubtitleConfig,
}

/// Subtitle configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleConfig {
    /// Enable subtitles by default
    pub enabled: bool,
    
    /// Font size
    pub font_size: u32,
    
    /// Font color (hex)
    pub color: String,
    
    /// Background opacity (0.0 - 1.0)
    pub background_opacity: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            window: WindowConfig::default(),
            decoder: DecoderConfig::default(),
            audio: AudioConfig::default(),
            general: GeneralConfig::default(),
        }
    }
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 720,
            fullscreen: false,
            title: "CCPlayer".to_string(),
            always_on_top: false,
            start_minimized: false,
        }
    }
}

impl Default for DecoderConfig {
    fn default() -> Self {
        Self {
            hardware_acceleration: true,
            thread_count: 0, // 0 = auto-detect
            buffer_size: 3,
            preferred_codecs: vec![
                "h264".to_string(),
                "hevc".to_string(),
                "vp9".to_string(),
                "av1".to_string(),
            ],
        }
    }
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            volume: 0.7,
            buffer_size: 512,
            sample_rate: 0, // 0 = auto-detect
            normalize: false,
        }
    }
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            remember_window_state: true,
            log_level: "info".to_string(),
            recent_files_limit: 10,
            auto_play: true,
            subtitles: SubtitleConfig::default(),
        }
    }
}

impl Default for SubtitleConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            font_size: 24,
            color: "#FFFFFF".to_string(),
            background_opacity: 0.7,
        }
    }
}

impl Config {
    /// Load configuration from various sources
    /// 
    /// Configuration is loaded in the following order (later sources override earlier):
    /// 1. Default values
    /// 2. System config file (/etc/ccplayer/config.toml on Linux)
    /// 3. User config file (~/.config/ccplayer/config.toml on Linux)
    /// 4. Environment variables (CCPLAYER_* prefix)
    pub fn load() -> Result<Self> {
        let mut config = Self::default();
        
        // Try to load system config
        if let Some(system_path) = Self::system_config_path() {
            if system_path.exists() {
                config.merge_from_file(&system_path)?;
            }
        }
        
        // Try to load user config
        if let Some(user_path) = Self::user_config_path() {
            if user_path.exists() {
                config.merge_from_file(&user_path)?;
            }
        }
        
        // Apply environment variable overrides
        config.apply_env_overrides()?;
        
        // Validate configuration
        config.validate()?;
        
        Ok(config)
    }
    
    /// Save configuration to user config file
    pub fn save(&self) -> Result<()> {
        let path = Self::user_config_path()
            .ok_or_else(|| CCPlayerError::Config("Cannot determine user config path".to_string()))?;
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| CCPlayerError::Config(format!("Failed to create config directory: {}", e)))?;
        }
        
        let toml = toml::to_string_pretty(self)
            .map_err(|e| CCPlayerError::Config(format!("Failed to serialize config: {}", e)))?;
        
        std::fs::write(&path, toml)
            .map_err(|e| CCPlayerError::Config(format!("Failed to write config file: {}", e)))?;
        
        Ok(())
    }
    
    /// Merge configuration from a TOML file
    fn merge_from_file(&mut self, path: &Path) -> Result<()> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| CCPlayerError::Config(format!("Failed to read config file: {}", e)))?;
        
        let file_config: Config = toml::from_str(&contents)
            .map_err(|e| CCPlayerError::Config(format!("Failed to parse config file: {}", e)))?;
        
        // TODO: Implement proper merging logic instead of full replacement
        *self = file_config;
        
        Ok(())
    }
    
    /// Apply environment variable overrides
    fn apply_env_overrides(&mut self) -> Result<()> {
        // Example: CCPLAYER_WINDOW_WIDTH=1920
        if let Ok(width) = std::env::var("CCPLAYER_WINDOW_WIDTH") {
            self.window.width = width.parse()
                .map_err(|_| CCPlayerError::Config("Invalid CCPLAYER_WINDOW_WIDTH".to_string()))?;
        }
        
        if let Ok(height) = std::env::var("CCPLAYER_WINDOW_HEIGHT") {
            self.window.height = height.parse()
                .map_err(|_| CCPlayerError::Config("Invalid CCPLAYER_WINDOW_HEIGHT".to_string()))?;
        }
        
        if let Ok(volume) = std::env::var("CCPLAYER_AUDIO_VOLUME") {
            self.audio.volume = volume.parse()
                .map_err(|_| CCPlayerError::Config("Invalid CCPLAYER_AUDIO_VOLUME".to_string()))?;
        }
        
        if let Ok(log_level) = std::env::var("CCPLAYER_LOG_LEVEL") {
            self.general.log_level = log_level;
        }
        
        Ok(())
    }
    
    /// Validate configuration values
    fn validate(&self) -> Result<()> {
        // Validate window dimensions
        if self.window.width == 0 || self.window.height == 0 {
            return Err(CCPlayerError::Config("Window dimensions must be non-zero".to_string()));
        }
        
        // Validate audio volume
        if !(0.0..=1.0).contains(&self.audio.volume) {
            return Err(CCPlayerError::Config("Audio volume must be between 0.0 and 1.0".to_string()));
        }
        
        // Validate log level
        let valid_log_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_log_levels.contains(&self.general.log_level.as_str()) {
            return Err(CCPlayerError::Config(format!(
                "Invalid log level '{}', must be one of: {:?}", 
                self.general.log_level, 
                valid_log_levels
            )));
        }
        
        Ok(())
    }
    
    /// Get system config file path
    fn system_config_path() -> Option<PathBuf> {
        #[cfg(target_os = "linux")]
        return Some(PathBuf::from("/etc/ccplayer/config.toml"));
        
        #[cfg(target_os = "windows")]
        return std::env::var("PROGRAMDATA").ok()
            .map(|p| PathBuf::from(p).join("CCPlayer").join("config.toml"));
        
        #[cfg(target_os = "macos")]
        return Some(PathBuf::from("/Library/Application Support/CCPlayer/config.toml"));
        
        #[allow(unreachable_code)]
        None
    }
    
    /// Get user config file path
    fn user_config_path() -> Option<PathBuf> {
        #[cfg(target_os = "linux")]
        return dirs::config_dir()
            .map(|p| p.join("ccplayer").join("config.toml"));
        
        #[cfg(target_os = "windows")]
        return dirs::config_dir()
            .map(|p| p.join("CCPlayer").join("config.toml"));
        
        #[cfg(target_os = "macos")]
        return dirs::config_dir()
            .map(|p| p.join("CCPlayer").join("config.toml"));
        
        #[allow(unreachable_code)]
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.window.width, 1280);
        assert_eq!(config.window.height, 720);
        assert!(!config.window.fullscreen);
        assert_eq!(config.audio.volume, 0.7);
    }
    
    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        assert!(config.validate().is_ok());
        
        config.window.width = 0;
        assert!(config.validate().is_err());
        
        config.window.width = 1280;
        config.audio.volume = 1.5;
        assert!(config.validate().is_err());
        
        config.audio.volume = 0.5;
        config.general.log_level = "invalid".to_string();
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml = toml::to_string(&config).unwrap();
        let deserialized: Config = toml::from_str(&toml).unwrap();
        
        assert_eq!(config.window.width, deserialized.window.width);
        assert_eq!(config.audio.volume, deserialized.audio.volume);
    }
}