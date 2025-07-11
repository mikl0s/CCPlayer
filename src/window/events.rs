//! Event handling for the winit window
//! 
//! This module converts winit events to CCPlayer WindowEvent types
//! and handles general window event processing.

use crate::window::{WindowEvent, WindowMetrics, MouseButton, Key, KeyModifiers};
use winit::event::{
    WindowEvent as WinitWindowEvent, 
    ElementState, 
    MouseButton as WinitMouseButton,
    MouseScrollDelta,
    ModifiersState,
};
use winit::keyboard::{KeyCode, PhysicalKey};

/// Event handler for converting winit events to CCPlayer events
pub struct EventHandler {
    /// Current keyboard modifiers state
    modifiers: ModifiersState,
    
    /// Last known mouse position
    mouse_position: (f64, f64),
}

impl EventHandler {
    /// Create a new event handler
    pub fn new() -> Self {
        Self {
            modifiers: ModifiersState::empty(),
            mouse_position: (0.0, 0.0),
        }
    }
    
    /// Handle a winit window event and convert to CCPlayer event
    pub fn handle_event(&mut self, event: WinitWindowEvent, metrics: &WindowMetrics) -> Option<WindowEvent> {
        match event {
            WinitWindowEvent::CloseRequested => Some(WindowEvent::CloseRequested),
            
            WinitWindowEvent::Resized(size) => Some(WindowEvent::Resized {
                width: size.width,
                height: size.height,
            }),
            
            WinitWindowEvent::Moved(position) => Some(WindowEvent::Moved {
                x: position.x,
                y: position.y,
            }),
            
            WinitWindowEvent::Focused(focused) => {
                Some(if focused {
                    WindowEvent::Focused
                } else {
                    WindowEvent::Unfocused
                })
            }
            
            WinitWindowEvent::CursorMoved { position, .. } => {
                self.mouse_position = (position.x, position.y);
                Some(WindowEvent::MouseMove {
                    x: position.x,
                    y: position.y,
                })
            }
            
            WinitWindowEvent::MouseInput { state, button, .. } => {
                if state == ElementState::Pressed {
                    let button = convert_mouse_button(button)?;
                    Some(WindowEvent::MouseClick {
                        x: self.mouse_position.0,
                        y: self.mouse_position.1,
                        button,
                    })
                } else {
                    None
                }
            }
            
            WinitWindowEvent::MouseWheel { delta, .. } => {
                let scroll_delta = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 120.0, // Standard wheel delta
                };
                
                // Volume control: scroll up increases, scroll down decreases
                Some(WindowEvent::MouseWheel {
                    delta: scroll_delta,
                })
            }
            
            WinitWindowEvent::ModifiersChanged(new_modifiers) => {
                self.modifiers = new_modifiers.state();
                None // Don't emit event for modifier changes alone
            }
            
            WinitWindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(keycode) = event.physical_key {
                    let key = convert_key_code(keycode)?;
                    let modifiers = convert_modifiers(self.modifiers);
                    
                    match event.state {
                        ElementState::Pressed => Some(WindowEvent::KeyPressed { key, modifiers }),
                        ElementState::Released => Some(WindowEvent::KeyReleased { key, modifiers }),
                    }
                } else {
                    None
                }
            }
            
            WinitWindowEvent::DroppedFile(path) => {
                Some(WindowEvent::FilesDropped {
                    paths: vec![path],
                })
            }
            
            // Window state events
            WinitWindowEvent::Occluded(occluded) => {
                if occluded {
                    Some(WindowEvent::Minimized)
                } else {
                    Some(WindowEvent::Restored)
                }
            }
            
            _ => None,
        }
    }
    
    /// Check if a control event occurred (titlebar button clicks)
    pub fn check_control_event(&self, x: f64, y: f64, metrics: &WindowMetrics) -> Option<WindowEvent> {
        // Check if click is in titlebar area
        if y <= metrics.titlebar_height as f64 {
            // Check close button (rightmost)
            let close_x = metrics.width as f64 - metrics.control_size as f64;
            if x >= close_x {
                return Some(WindowEvent::ControlEvent(crate::window::ControlEvent::Close));
            }
            
            // Check maximize button
            let max_x = close_x - metrics.control_size as f64;
            if x >= max_x && x < close_x {
                return Some(WindowEvent::ControlEvent(crate::window::ControlEvent::Maximize));
            }
            
            // Check minimize button
            let min_x = max_x - metrics.control_size as f64;
            if x >= min_x && x < max_x {
                return Some(WindowEvent::ControlEvent(crate::window::ControlEvent::Minimize));
            }
        }
        
        None
    }
    
    /// Get current modifiers state
    pub fn modifiers(&self) -> ModifiersState {
        self.modifiers
    }
    
    /// Get current mouse position
    pub fn mouse_position(&self) -> (f64, f64) {
        self.mouse_position
    }
}

/// Convert winit mouse button to CCPlayer mouse button
fn convert_mouse_button(button: WinitMouseButton) -> Option<MouseButton> {
    match button {
        WinitMouseButton::Left => Some(MouseButton::Left),
        WinitMouseButton::Right => Some(MouseButton::Right),
        WinitMouseButton::Middle => Some(MouseButton::Middle),
        _ => None,
    }
}

/// Convert winit key code to CCPlayer key
fn convert_key_code(keycode: KeyCode) -> Option<Key> {
    match keycode {
        // Media controls
        KeyCode::Space => Some(Key::Space),
        KeyCode::Enter => Some(Key::Enter),
        KeyCode::Escape => Some(Key::Escape),
        
        // Navigation
        KeyCode::ArrowLeft => Some(Key::Left),
        KeyCode::ArrowRight => Some(Key::Right),
        KeyCode::ArrowUp => Some(Key::Up),
        KeyCode::ArrowDown => Some(Key::Down),
        
        // Seek
        KeyCode::PageUp => Some(Key::PageUp),
        KeyCode::PageDown => Some(Key::PageDown),
        KeyCode::Home => Some(Key::Home),
        KeyCode::End => Some(Key::End),
        
        // Volume (media keys)
        KeyCode::AudioVolumeUp => Some(Key::VolumeUp),
        KeyCode::AudioVolumeDown => Some(Key::VolumeDown),
        KeyCode::AudioVolumeMute => Some(Key::VolumeMute),
        
        // Playback speed
        KeyCode::Minus => Some(Key::Minus),
        KeyCode::Equal => Some(Key::Plus), // Plus is typically on the equals key
        
        // Letters
        KeyCode::KeyF => Some(Key::F),
        KeyCode::KeyM => Some(Key::M),
        KeyCode::KeyO => Some(Key::O),
        KeyCode::KeyQ => Some(Key::Q),
        KeyCode::KeyS => Some(Key::S),
        
        // Numbers
        KeyCode::Digit0 => Some(Key::Num0),
        KeyCode::Digit1 => Some(Key::Num1),
        KeyCode::Digit2 => Some(Key::Num2),
        KeyCode::Digit3 => Some(Key::Num3),
        KeyCode::Digit4 => Some(Key::Num4),
        KeyCode::Digit5 => Some(Key::Num5),
        KeyCode::Digit6 => Some(Key::Num6),
        KeyCode::Digit7 => Some(Key::Num7),
        KeyCode::Digit8 => Some(Key::Num8),
        KeyCode::Digit9 => Some(Key::Num9),
        
        // Other keys can be mapped as needed
        _ => None,
    }
}

/// Convert winit modifiers to CCPlayer modifiers
fn convert_modifiers(modifiers: ModifiersState) -> KeyModifiers {
    KeyModifiers {
        shift: modifiers.shift_key(),
        ctrl: modifiers.control_key(),
        alt: modifiers.alt_key(),
        meta: modifiers.super_key(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_event_handler_creation() {
        let handler = EventHandler::new();
        assert_eq!(handler.mouse_position(), (0.0, 0.0));
        assert!(handler.modifiers().is_empty());
    }
    
    #[test]
    fn test_mouse_button_conversion() {
        assert_eq!(convert_mouse_button(WinitMouseButton::Left), Some(MouseButton::Left));
        assert_eq!(convert_mouse_button(WinitMouseButton::Right), Some(MouseButton::Right));
        assert_eq!(convert_mouse_button(WinitMouseButton::Middle), Some(MouseButton::Middle));
    }
    
    #[test]
    fn test_key_conversion() {
        assert_eq!(convert_key_code(KeyCode::Space), Some(Key::Space));
        assert_eq!(convert_key_code(KeyCode::KeyF), Some(Key::F));
        assert_eq!(convert_key_code(KeyCode::Digit0), Some(Key::Num0));
        assert_eq!(convert_key_code(KeyCode::ArrowLeft), Some(Key::Left));
    }
    
    #[test]
    fn test_modifiers_conversion() {
        let mut mods = ModifiersState::empty();
        mods.set(ModifiersState::SHIFT, true);
        mods.set(ModifiersState::CONTROL, true);
        
        let converted = convert_modifiers(mods);
        assert!(converted.shift);
        assert!(converted.ctrl);
        assert!(!converted.alt);
        assert!(!converted.meta);
    }
}