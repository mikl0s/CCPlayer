//! Alt+drag window movement implementation
//! 
//! This module handles the Alt + left-click drag functionality
//! for moving the borderless window.

use crate::window::WindowEvent;
use winit::{
    event::{WindowEvent as WinitWindowEvent, ElementState, MouseButton},
    window::Window as WinitWindow,
    dpi::Position,
};
use std::sync::Arc;

/// Handler for window dragging functionality
pub struct DragHandler {
    /// Whether we're currently dragging
    is_dragging: bool,
    
    /// Whether Alt key is held
    alt_held: bool,
    
    /// Initial mouse position when drag started
    drag_start_pos: Option<(f64, f64)>,
    
    /// Initial window position when drag started
    window_start_pos: Option<(i32, i32)>,
}

impl DragHandler {
    /// Create a new drag handler
    pub fn new() -> Self {
        Self {
            is_dragging: false,
            alt_held: false,
            drag_start_pos: None,
            window_start_pos: None,
        }
    }
    
    /// Handle window event for drag functionality
    pub fn handle_event(&mut self, event: &WinitWindowEvent, window: &Arc<WinitWindow>) -> Option<WindowEvent> {
        match event {
            WinitWindowEvent::ModifiersChanged(modifiers) => {
                self.alt_held = modifiers.state().alt_key();
                
                // Stop dragging if Alt is released
                if !self.alt_held && self.is_dragging {
                    self.stop_dragging();
                }
                None
            }
            
            WinitWindowEvent::MouseInput { state, button, .. } => {
                if *button == MouseButton::Left && self.alt_held {
                    match state {
                        ElementState::Pressed => {
                            self.start_dragging(window);
                            None
                        }
                        ElementState::Released => {
                            self.stop_dragging();
                            None
                        }
                    }
                } else {
                    None
                }
            }
            
            WinitWindowEvent::CursorMoved { position, .. } => {
                if self.is_dragging {
                    self.handle_drag(position.x, position.y, window);
                    
                    // Return a mouse move event to indicate we handled the drag
                    Some(WindowEvent::MouseMove {
                        x: position.x,
                        y: position.y,
                    })
                } else {
                    None
                }
            }
            
            // Stop dragging if window loses focus
            WinitWindowEvent::Focused(false) => {
                if self.is_dragging {
                    self.stop_dragging();
                }
                None
            }
            
            _ => None,
        }
    }
    
    /// Start dragging the window
    fn start_dragging(&mut self, window: &Arc<WinitWindow>) {
        self.is_dragging = true;
        
        // Get current cursor position (screen coordinates)
        if let Ok(pos) = window.cursor_position() {
            if let Ok(window_pos) = window.outer_position() {
                self.drag_start_pos = Some((pos.x, pos.y));
                self.window_start_pos = Some((window_pos.x, window_pos.y));
            }
        }
    }
    
    /// Stop dragging the window
    fn stop_dragging(&mut self) {
        self.is_dragging = false;
        self.drag_start_pos = None;
        self.window_start_pos = None;
    }
    
    /// Handle drag movement
    fn handle_drag(&mut self, cursor_x: f64, cursor_y: f64, window: &Arc<WinitWindow>) {
        if let (Some(drag_start), Some(window_start)) = (self.drag_start_pos, self.window_start_pos) {
            // Calculate the delta from the drag start position
            let delta_x = cursor_x - drag_start.0;
            let delta_y = cursor_y - drag_start.1;
            
            // Calculate new window position
            let new_x = window_start.0 + delta_x as i32;
            let new_y = window_start.1 + delta_y as i32;
            
            // Move the window
            let _ = window.set_outer_position(Position::Physical((new_x, new_y).into()));
        }
    }
    
    /// Check if currently dragging
    pub fn is_dragging(&self) -> bool {
        self.is_dragging
    }
    
    /// Check if Alt key is held
    pub fn is_alt_held(&self) -> bool {
        self.alt_held
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_drag_handler_creation() {
        let handler = DragHandler::new();
        assert!(!handler.is_dragging());
        assert!(!handler.is_alt_held());
    }
    
    #[test]
    fn test_drag_state_management() {
        let mut handler = DragHandler::new();
        
        // Initially not dragging
        assert!(!handler.is_dragging);
        
        // Simulate starting drag
        handler.alt_held = true;
        handler.is_dragging = true;
        handler.drag_start_pos = Some((100.0, 100.0));
        handler.window_start_pos = Some((50, 50));
        
        assert!(handler.is_dragging());
        
        // Stop dragging
        handler.stop_dragging();
        assert!(!handler.is_dragging());
        assert!(handler.drag_start_pos.is_none());
        assert!(handler.window_start_pos.is_none());
    }
}