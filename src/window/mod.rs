//! Window management module for CCPlayer
//! 
//! This module handles window creation, event handling, and UI controls
//! for the media player. It provides a borderless window with custom
//! controls and drag functionality.

use crate::utils::error::Result;
use crate::utils::config::WindowConfig;
use std::sync::Arc;
use winit::event::WindowEvent as WinitWindowEvent;

// Re-export the winit window implementation
pub mod winit_window;
pub use winit_window::WinitWindowImpl;

/// Window trait defining the interface for window implementations
pub trait Window: Send + Sync {
    /// Create a new window with the given configuration
    /// 
    /// # Arguments
    /// 
    /// * `config` - Window configuration
    /// 
    /// # Returns
    /// 
    /// Returns the window instance or an error
    fn new(config: WindowConfig) -> Result<Self> where Self: Sized;
    
    /// Show the window
    fn show(&mut self) -> Result<()>;
    
    /// Hide the window
    fn hide(&mut self) -> Result<()>;
    
    /// Set the window title
    /// 
    /// # Arguments
    /// 
    /// * `title` - New window title
    fn set_title(&mut self, title: &str) -> Result<()>;
    
    /// Set fullscreen mode
    /// 
    /// # Arguments
    /// 
    /// * `fullscreen` - Whether to enable fullscreen
    fn set_fullscreen(&mut self, fullscreen: bool) -> Result<()>;
    
    /// Get current fullscreen state
    fn is_fullscreen(&self) -> bool;
    
    /// Handle window events
    /// 
    /// # Returns
    /// 
    /// Returns a vector of processed window events
    fn handle_events(&mut self) -> Result<Vec<WindowEvent>>;
    
    /// Get the window handle for renderer initialization
    fn handle(&self) -> &dyn std::any::Any;
    
    /// Get the raw window handle for wgpu surface creation
    fn raw_window_handle(&self) -> Result<raw_window_handle::WindowHandle<'_>>;
    
    /// Get the current window size
    fn size(&self) -> (u32, u32);
}

/// Window events that can occur
#[derive(Debug, Clone, PartialEq)]
pub enum WindowEvent {
    /// Window close requested
    CloseRequested,
    
    /// Window resized
    Resized { width: u32, height: u32 },
    
    /// Window moved
    Moved { x: i32, y: i32 },
    
    /// Window minimized
    Minimized,
    
    /// Window maximized
    Maximized,
    
    /// Window restored from minimized/maximized
    Restored,
    
    /// Window gained focus
    Focused,
    
    /// Window lost focus
    Unfocused,
    
    /// Mouse clicked at position
    MouseClick { x: f64, y: f64, button: MouseButton },
    
    /// Mouse moved to position
    MouseMove { x: f64, y: f64 },
    
    /// Mouse wheel scrolled
    MouseWheel { delta: f32 },
    
    /// Key pressed
    KeyPressed { key: Key, modifiers: KeyModifiers },
    
    /// Key released
    KeyReleased { key: Key, modifiers: KeyModifiers },
    
    /// File(s) dropped onto window
    FilesDropped { paths: Vec<std::path::PathBuf> },
    
    /// Custom control event (minimize, maximize, close buttons)
    ControlEvent(ControlEvent),
}

/// Mouse button types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Keyboard key types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    // Media controls
    Space,
    Enter,
    Escape,
    
    // Navigation
    Left,
    Right,
    Up,
    Down,
    
    // Seek
    PageUp,
    PageDown,
    Home,
    End,
    
    // Volume
    VolumeUp,
    VolumeDown,
    VolumeMute,
    
    // Playback speed
    Minus,
    Plus,
    
    // Other
    F,  // Fullscreen
    M,  // Mute
    O,  // Open file
    Q,  // Quit
    S,  // Subtitles
    
    // Numbers 0-9
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
    
    // Other keys can be added as needed
    Other(String),
}

/// Keyboard modifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyModifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,  // Windows/Super/Command key
}

impl Default for KeyModifiers {
    fn default() -> Self {
        Self {
            shift: false,
            ctrl: false,
            alt: false,
            meta: false,
        }
    }
}

/// Window control events (custom titlebar buttons)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlEvent {
    Minimize,
    Maximize,
    Close,
}

/// Window metrics for layout calculations
#[derive(Debug, Clone, Copy)]
pub struct WindowMetrics {
    /// Total window width
    pub width: u32,
    
    /// Total window height
    pub height: u32,
    
    /// Titlebar height
    pub titlebar_height: u32,
    
    /// Control button size
    pub control_size: u32,
    
    /// DPI scale factor
    pub scale_factor: f64,
}

impl Default for WindowMetrics {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 720,
            titlebar_height: 32,
            control_size: 46,
            scale_factor: 1.0,
        }
    }
}

/// Convert winit window events to our window events
pub fn convert_winit_event(event: WinitWindowEvent) -> Option<WindowEvent> {
    match event {
        WinitWindowEvent::CloseRequested => Some(WindowEvent::CloseRequested),
        WinitWindowEvent::Resized(size) => Some(WindowEvent::Resized {
            width: size.width,
            height: size.height,
        }),
        WinitWindowEvent::Focused(focused) => {
            Some(if focused {
                WindowEvent::Focused
            } else {
                WindowEvent::Unfocused
            })
        }
        // Add more conversions as needed
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_key_modifiers_default() {
        let mods = KeyModifiers::default();
        assert!(!mods.shift);
        assert!(!mods.ctrl);
        assert!(!mods.alt);
        assert!(!mods.meta);
    }
    
    #[test]
    fn test_window_metrics_default() {
        let metrics = WindowMetrics::default();
        assert_eq!(metrics.width, 1280);
        assert_eq!(metrics.height, 720);
        assert_eq!(metrics.titlebar_height, 32);
        assert_eq!(metrics.control_size, 46);
        assert_eq!(metrics.scale_factor, 1.0);
    }
}