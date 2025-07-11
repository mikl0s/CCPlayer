//! Error types for CCPlayer
//! 
//! This module defines custom error types used throughout the application.
//! We use thiserror for convenient error type definitions and anyhow for
//! application-level error handling.

use thiserror::Error;

/// Main error type for CCPlayer
#[derive(Error, Debug)]
pub enum CCPlayerError {
    /// Window-related errors
    #[error("Window error: {0}")]
    Window(String),
    
    /// Renderer errors
    #[error("Renderer error: {0}")]
    Renderer(String),
    
    /// Decoder errors
    #[error("Decoder error: {0}")]
    Decoder(String),
    
    /// Audio errors
    #[error("Audio error: {0}")]
    Audio(String),
    
    /// Configuration errors
    #[error("Configuration error: {0}")]
    Config(String),
    
    /// File I/O errors
    #[error("File error: {0}")]
    FileIO(#[from] std::io::Error),
    
    /// Invalid input errors
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    
    /// Resource not found
    #[error("Resource not found: {0}")]
    NotFound(String),
    
    /// Unsupported format
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
    
    /// Synchronization error
    #[error("Synchronization error: {0}")]
    Sync(String),
    
    /// Generic error for unexpected situations
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<ffmpeg_next::Error> for CCPlayerError {
    fn from(err: ffmpeg_next::Error) -> Self {
        CCPlayerError::Decoder(format!("FFmpeg error: {}", err))
    }
}

impl From<std::ffi::NulError> for CCPlayerError {
    fn from(err: std::ffi::NulError) -> Self {
        CCPlayerError::Decoder(format!("FFI string error: {}", err))
    }
}

impl CCPlayerError {
    /// Create a decoder error from string
    pub fn decoder_error<S: Into<String>>(msg: S) -> Self {
        CCPlayerError::Decoder(msg.into())
    }
}

/// Convenience type alias for Results in CCPlayer
pub type Result<T> = std::result::Result<T, CCPlayerError>;

/// Extension trait for converting other errors to CCPlayerError
pub trait IntoPlayerError<T> {
    /// Convert this error into a CCPlayerError with the given context
    fn window_err(self, context: &str) -> Result<T>;
    fn renderer_err(self, context: &str) -> Result<T>;
    fn decoder_err(self, context: &str) -> Result<T>;
    fn audio_err(self, context: &str) -> Result<T>;
    fn config_err(self, context: &str) -> Result<T>;
}

impl<T, E: std::fmt::Display> IntoPlayerError<T> for std::result::Result<T, E> {
    fn window_err(self, context: &str) -> Result<T> {
        self.map_err(|e| CCPlayerError::Window(format!("{}: {}", context, e)))
    }
    
    fn renderer_err(self, context: &str) -> Result<T> {
        self.map_err(|e| CCPlayerError::Renderer(format!("{}: {}", context, e)))
    }
    
    fn decoder_err(self, context: &str) -> Result<T> {
        self.map_err(|e| CCPlayerError::Decoder(format!("{}: {}", context, e)))
    }
    
    fn audio_err(self, context: &str) -> Result<T> {
        self.map_err(|e| CCPlayerError::Audio(format!("{}: {}", context, e)))
    }
    
    fn config_err(self, context: &str) -> Result<T> {
        self.map_err(|e| CCPlayerError::Config(format!("{}: {}", context, e)))
    }
}

/// Helper macro for creating internal errors with file and line information
#[macro_export]
macro_rules! internal_error {
    ($msg:expr) => {
        $crate::utils::error::CCPlayerError::Internal(
            format!("{} at {}:{}", $msg, file!(), line!())
        )
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::utils::error::CCPlayerError::Internal(
            format!("{} at {}:{}", format!($fmt, $($arg)*), file!(), line!())
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_display() {
        let err = CCPlayerError::Window("Failed to create window".to_string());
        assert_eq!(err.to_string(), "Window error: Failed to create window");
        
        let err = CCPlayerError::UnsupportedFormat("MP4".to_string());
        assert_eq!(err.to_string(), "Unsupported format: MP4");
    }
    
    #[test]
    fn test_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let player_err: CCPlayerError = io_err.into();
        assert!(matches!(player_err, CCPlayerError::FileIO(_)));
    }
    
    #[test]
    fn test_into_player_error_trait() {
        let result: std::result::Result<(), &str> = Err("Something went wrong");
        let converted = result.window_err("Creating surface");
        
        match converted {
            Err(CCPlayerError::Window(msg)) => {
                assert_eq!(msg, "Creating surface: Something went wrong");
            }
            _ => panic!("Expected Window error"),
        }
    }
}