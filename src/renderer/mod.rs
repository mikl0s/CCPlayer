//! Renderer module for CCPlayer
//! 
//! This module handles GPU-based rendering using wgpu. It manages the render
//! pipeline, texture uploads, and overlay compositing for the media player.

use crate::utils::error::Result;
use crate::window::Window;
use std::sync::Arc;

// Export submodules
pub mod frame;
pub mod pipeline;
pub mod texture;
pub mod wgpu_renderer;

// Re-export main types
pub use wgpu_renderer::WgpuRenderer;

/// Renderer trait defining the interface for video rendering
pub trait Renderer: Send + Sync {
    /// Create a new renderer for the given window
    /// 
    /// # Arguments
    /// 
    /// * `window` - Window to render to
    /// 
    /// # Returns
    /// 
    /// Returns the renderer instance or an error
    fn new(window: Arc<dyn Window>) -> Result<Self> where Self: Sized;
    
    /// Render a video frame
    /// 
    /// # Arguments
    /// 
    /// * `frame` - Video frame to render
    fn render_frame(&mut self, frame: VideoFrame) -> Result<()>;
    
    /// Render an overlay on top of the video
    /// 
    /// # Arguments
    /// 
    /// * `overlay` - Overlay to render
    fn render_overlay(&mut self, overlay: Overlay) -> Result<()>;
    
    /// Clear all overlays
    fn clear_overlays(&mut self) -> Result<()>;
    
    /// Present the rendered frame to the screen
    fn present(&mut self) -> Result<()>;
    
    /// Handle window resize
    /// 
    /// # Arguments
    /// 
    /// * `width` - New window width
    /// * `height` - New window height
    fn resize(&mut self, width: u32, height: u32) -> Result<()>;
    
    /// Set video aspect ratio for proper scaling
    /// 
    /// # Arguments
    /// 
    /// * `aspect_ratio` - Video aspect ratio (width / height)
    fn set_aspect_ratio(&mut self, aspect_ratio: f32) -> Result<()>;
    
    /// Take a screenshot of the current frame
    /// 
    /// # Returns
    /// 
    /// Returns the screenshot as RGBA8 data
    fn screenshot(&self) -> Result<Vec<u8>>;
}

/// Video frame data
#[derive(Debug, Clone)]
pub struct VideoFrame {
    /// Frame data in YUV or RGB format
    pub data: FrameData,
    
    /// Presentation timestamp in microseconds
    pub pts: i64,
    
    /// Frame duration in microseconds
    pub duration: i64,
    
    /// Frame width
    pub width: u32,
    
    /// Frame height
    pub height: u32,
    
    /// Pixel aspect ratio
    pub par: f32,
}

/// Frame data formats
#[derive(Debug, Clone)]
pub enum FrameData {
    /// YUV 4:2:0 planar format (most common for video)
    Yuv420 {
        y_plane: Vec<u8>,
        u_plane: Vec<u8>,
        v_plane: Vec<u8>,
        y_stride: usize,
        uv_stride: usize,
    },
    
    /// YUV 4:2:2 planar format
    Yuv422 {
        y_plane: Vec<u8>,
        u_plane: Vec<u8>,
        v_plane: Vec<u8>,
        y_stride: usize,
        uv_stride: usize,
    },
    
    /// YUV 4:4:4 planar format
    Yuv444 {
        y_plane: Vec<u8>,
        u_plane: Vec<u8>,
        v_plane: Vec<u8>,
        stride: usize,
    },
    
    /// RGB format (3 bytes per pixel)
    Rgb {
        data: Vec<u8>,
        stride: usize,
    },
    
    /// RGBA format (4 bytes per pixel)
    Rgba {
        data: Vec<u8>,
        stride: usize,
    },
    
    /// NV12 format (Y plane + interleaved UV)
    Nv12 {
        y_plane: Vec<u8>,
        uv_plane: Vec<u8>,
        y_stride: usize,
        uv_stride: usize,
    },
}

/// Overlay types that can be rendered on top of video
#[derive(Debug, Clone)]
pub enum Overlay {
    /// Volume indicator overlay
    Volume {
        level: f32,  // 0.0 to 1.0
        position: OverlayPosition,
        duration_ms: u32,
    },
    
    /// Playback controls overlay
    Controls {
        playing: bool,
        position: f64,  // 0.0 to 1.0
        duration: std::time::Duration,
        visible: bool,
    },
    
    /// Text overlay (subtitles, info, etc.)
    Text {
        content: String,
        position: OverlayPosition,
        font_size: u32,
        color: Color,
        background: Option<Color>,
    },
    
    /// Loading spinner
    Loading {
        position: OverlayPosition,
    },
    
    /// Custom image overlay
    Image {
        data: Vec<u8>,
        width: u32,
        height: u32,
        position: OverlayPosition,
        opacity: f32,
    },
}

/// Overlay positioning
#[derive(Debug, Clone, Copy)]
pub enum OverlayPosition {
    /// Centered on screen
    Center,
    
    /// Top-left corner with offset
    TopLeft { x: f32, y: f32 },
    
    /// Top-right corner with offset
    TopRight { x: f32, y: f32 },
    
    /// Bottom-left corner with offset
    BottomLeft { x: f32, y: f32 },
    
    /// Bottom-right corner with offset
    BottomRight { x: f32, y: f32 },
    
    /// Custom absolute position
    Absolute { x: f32, y: f32 },
    
    /// Custom relative position (0.0 to 1.0)
    Relative { x: f32, y: f32 },
}

/// Color representation
#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    /// Create a new color
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
    
    /// Create color from RGB values (0-255)
    pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: 1.0,
        }
    }
    
    /// Create color from RGBA values (0-255)
    pub fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        }
    }
    
    /// Create color from hex string
    pub fn from_hex(hex: &str) -> Result<Self> {
        let hex = hex.trim_start_matches('#');
        
        if hex.len() != 6 && hex.len() != 8 {
            return Err(crate::utils::error::CCPlayerError::InvalidInput(
                "Hex color must be 6 or 8 characters".to_string()
            ));
        }
        
        let r = u8::from_str_radix(&hex[0..2], 16)
            .map_err(|_| crate::utils::error::CCPlayerError::InvalidInput("Invalid hex color".to_string()))?;
        let g = u8::from_str_radix(&hex[2..4], 16)
            .map_err(|_| crate::utils::error::CCPlayerError::InvalidInput("Invalid hex color".to_string()))?;
        let b = u8::from_str_radix(&hex[4..6], 16)
            .map_err(|_| crate::utils::error::CCPlayerError::InvalidInput("Invalid hex color".to_string()))?;
        
        let a = if hex.len() == 8 {
            u8::from_str_radix(&hex[6..8], 16)
                .map_err(|_| crate::utils::error::CCPlayerError::InvalidInput("Invalid hex color".to_string()))?
        } else {
            255
        };
        
        Ok(Self::from_rgba(r, g, b, a))
    }
    
    // Common colors
    pub const WHITE: Self = Self { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
    pub const BLACK: Self = Self { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const RED: Self = Self { r: 1.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const GREEN: Self = Self { r: 0.0, g: 1.0, b: 0.0, a: 1.0 };
    pub const BLUE: Self = Self { r: 0.0, g: 0.0, b: 1.0, a: 1.0 };
    pub const TRANSPARENT: Self = Self { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };
}

/// Render statistics for performance monitoring
#[derive(Debug, Clone, Copy, Default)]
pub struct RenderStats {
    /// Frames rendered in the last second
    pub fps: f32,
    
    /// Average frame render time in milliseconds
    pub frame_time: f32,
    
    /// Number of dropped frames
    pub dropped_frames: u64,
    
    /// GPU memory usage in bytes
    pub gpu_memory: u64,
    
    /// Current render resolution
    pub render_width: u32,
    pub render_height: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_color_from_rgb() {
        let color = Color::from_rgb(255, 128, 0);
        assert_eq!(color.r, 1.0);
        assert_eq!(color.g, 128.0 / 255.0);
        assert_eq!(color.b, 0.0);
        assert_eq!(color.a, 1.0);
    }
    
    #[test]
    fn test_color_from_hex() {
        let color = Color::from_hex("#FF8000").unwrap();
        assert_eq!(color.r, 1.0);
        assert_eq!(color.g, 128.0 / 255.0);
        assert_eq!(color.b, 0.0);
        assert_eq!(color.a, 1.0);
        
        let color_with_alpha = Color::from_hex("#FF800080").unwrap();
        assert_eq!(color_with_alpha.a, 128.0 / 255.0);
        
        assert!(Color::from_hex("#GG0000").is_err());
        assert!(Color::from_hex("#FF00").is_err());
    }
    
    #[test]
    fn test_color_constants() {
        assert_eq!(Color::WHITE.r, 1.0);
        assert_eq!(Color::BLACK.r, 0.0);
        assert_eq!(Color::TRANSPARENT.a, 0.0);
    }
}