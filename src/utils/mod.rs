//! Utility module for CCPlayer
//! 
//! This module provides common utilities used throughout the application:
//! - Error handling with custom error types
//! - Configuration management
//! - Logging utilities
//! - Common helper functions

pub mod config;
pub mod error;

// Re-export commonly used items
pub use config::{Config, WindowConfig, DecoderConfig, AudioConfig};
pub use error::{CCPlayerError, Result};

/// Initialize the application configuration
/// 
/// Loads configuration from:
/// 1. Default values
/// 2. System configuration file
/// 3. User configuration file
/// 4. Environment variables
/// 
/// # Returns
/// 
/// Returns the loaded configuration or an error if loading fails
pub fn load_config() -> Result<Config> {
    Config::load()
}

/// Format a duration for display
/// 
/// # Arguments
/// 
/// * `duration` - Duration to format
/// 
/// # Returns
/// 
/// Formatted string in the format "HH:MM:SS" or "MM:SS" for durations under an hour
pub fn format_duration(duration: std::time::Duration) -> String {
    let total_secs = duration.as_secs();
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    
    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}", minutes, seconds)
    }
}

/// Clamp a value between min and max
/// 
/// # Arguments
/// 
/// * `value` - Value to clamp
/// * `min` - Minimum value
/// * `max` - Maximum value
/// 
/// # Returns
/// 
/// The clamped value
pub fn clamp<T: PartialOrd>(value: T, min: T, max: T) -> T {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_format_duration() {
        use std::time::Duration;
        
        assert_eq!(format_duration(Duration::from_secs(0)), "00:00");
        assert_eq!(format_duration(Duration::from_secs(59)), "00:59");
        assert_eq!(format_duration(Duration::from_secs(60)), "01:00");
        assert_eq!(format_duration(Duration::from_secs(3599)), "59:59");
        assert_eq!(format_duration(Duration::from_secs(3600)), "01:00:00");
        assert_eq!(format_duration(Duration::from_secs(7325)), "02:02:05");
    }
    
    #[test]
    fn test_clamp() {
        assert_eq!(clamp(5, 0, 10), 5);
        assert_eq!(clamp(-5, 0, 10), 0);
        assert_eq!(clamp(15, 0, 10), 10);
        assert_eq!(clamp(0.5, 0.0, 1.0), 0.5);
        assert_eq!(clamp(-0.5, 0.0, 1.0), 0.0);
        assert_eq!(clamp(1.5, 0.0, 1.0), 1.0);
    }
}