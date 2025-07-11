//! Edge resize handling for borderless windows
//! 
//! This module detects when the cursor is near window edges and
//! handles resize operations with appropriate cursor feedback.

use crate::window::{WindowEvent, WindowMetrics};
use winit::{
    event::{WindowEvent as WinitWindowEvent, ElementState, MouseButton},
    window::{Window as WinitWindow, CursorIcon},
    dpi::{LogicalSize, LogicalPosition},
};
use std::sync::Arc;

/// Resize edge enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeEdge {
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
    TopLeft,
}

impl ResizeEdge {
    /// Convert to cursor icon
    pub fn to_cursor_icon(&self) -> CursorIcon {
        match self {
            ResizeEdge::Top | ResizeEdge::Bottom => CursorIcon::NsResize,
            ResizeEdge::Left | ResizeEdge::Right => CursorIcon::EwResize,
            ResizeEdge::TopLeft | ResizeEdge::BottomRight => CursorIcon::NwseResize,
            ResizeEdge::TopRight | ResizeEdge::BottomLeft => CursorIcon::NeswResize,
        }
    }
}

/// Handler for window resizing functionality
pub struct ResizeHandler {
    /// Border width for edge detection (in pixels)
    border_width: u32,
    
    /// Current resize edge (if any)
    current_edge: Option<ResizeEdge>,
    
    /// Whether we're currently resizing
    is_resizing: bool,
    
    /// Initial mouse position when resize started
    resize_start_pos: Option<(f64, f64)>,
    
    /// Initial window size when resize started
    window_start_size: Option<(u32, u32)>,
    
    /// Initial window position when resize started (for edges that move the window)
    window_start_pos: Option<(i32, i32)>,
}

impl ResizeHandler {
    /// Create a new resize handler
    pub fn new(border_width: u32) -> Self {
        Self {
            border_width,
            current_edge: None,
            is_resizing: false,
            resize_start_pos: None,
            window_start_size: None,
            window_start_pos: None,
        }
    }
    
    /// Handle window event for resize functionality
    pub fn handle_event(
        &mut self, 
        event: &WinitWindowEvent, 
        window: &Arc<WinitWindow>,
        metrics: &WindowMetrics
    ) -> Option<(WindowEvent, CursorIcon)> {
        match event {
            WinitWindowEvent::CursorMoved { position, .. } => {
                let edge = self.hit_test(position.x, position.y, metrics);
                
                if self.is_resizing {
                    // Handle resize movement
                    self.handle_resize(position.x, position.y, window);
                    
                    // Keep the resize cursor during resize
                    let cursor = self.current_edge
                        .map(|e| e.to_cursor_icon())
                        .unwrap_or(CursorIcon::Default);
                    
                    return Some((WindowEvent::MouseMove {
                        x: position.x,
                        y: position.y,
                    }, cursor));
                } else {
                    // Update cursor based on edge detection
                    self.current_edge = edge;
                    let cursor = edge
                        .map(|e| e.to_cursor_icon())
                        .unwrap_or(CursorIcon::Default);
                    
                    return Some((WindowEvent::MouseMove {
                        x: position.x,
                        y: position.y,
                    }, cursor));
                }
            }
            
            WinitWindowEvent::MouseInput { state, button, .. } => {
                if *button == MouseButton::Left {
                    match state {
                        ElementState::Pressed => {
                            if let Some(edge) = self.current_edge {
                                self.start_resize(edge, window);
                                return None; // Don't propagate the click
                            }
                        }
                        ElementState::Released => {
                            if self.is_resizing {
                                self.stop_resize();
                                return None;
                            }
                        }
                    }
                }
                None
            }
            
            WinitWindowEvent::CursorLeft { .. } => {
                // Reset cursor when leaving window
                self.current_edge = None;
                if !self.is_resizing {
                    return Some((WindowEvent::MouseMove { x: -1.0, y: -1.0 }, CursorIcon::Default));
                }
                None
            }
            
            WinitWindowEvent::Focused(false) => {
                // Stop resizing if window loses focus
                if self.is_resizing {
                    self.stop_resize();
                }
                None
            }
            
            _ => None,
        }
    }
    
    /// Perform hit testing to determine which edge the cursor is near
    pub fn hit_test(&self, x: f64, y: f64, metrics: &WindowMetrics) -> Option<ResizeEdge> {
        let width = metrics.width as f64;
        let height = metrics.height as f64;
        let border = self.border_width as f64;
        
        // Don't allow resize from titlebar area
        if y <= metrics.titlebar_height as f64 {
            return None;
        }
        
        // Check corners first (they take priority)
        if x <= border && y <= border {
            return Some(ResizeEdge::TopLeft);
        }
        if x >= width - border && y <= border {
            return Some(ResizeEdge::TopRight);
        }
        if x <= border && y >= height - border {
            return Some(ResizeEdge::BottomLeft);
        }
        if x >= width - border && y >= height - border {
            return Some(ResizeEdge::BottomRight);
        }
        
        // Check edges
        if x <= border {
            return Some(ResizeEdge::Left);
        }
        if x >= width - border {
            return Some(ResizeEdge::Right);
        }
        if y <= border {
            return Some(ResizeEdge::Top);
        }
        if y >= height - border {
            return Some(ResizeEdge::Bottom);
        }
        
        None
    }
    
    /// Start resizing from the given edge
    fn start_resize(&mut self, edge: ResizeEdge, window: &Arc<WinitWindow>) {
        self.is_resizing = true;
        self.current_edge = Some(edge);
        
        // Store initial state
        if let Ok(pos) = window.cursor_position() {
            self.resize_start_pos = Some((pos.x, pos.y));
        }
        
        let size = window.inner_size();
        self.window_start_size = Some((size.width, size.height));
        
        if let Ok(pos) = window.outer_position() {
            self.window_start_pos = Some((pos.x, pos.y));
        }
    }
    
    /// Stop resizing
    fn stop_resize(&mut self) {
        self.is_resizing = false;
        self.resize_start_pos = None;
        self.window_start_size = None;
        self.window_start_pos = None;
    }
    
    /// Handle resize movement
    fn handle_resize(&self, cursor_x: f64, cursor_y: f64, window: &Arc<WinitWindow>) {
        if let (Some(edge), Some(start_pos), Some(start_size), Some(window_pos)) = 
            (self.current_edge, self.resize_start_pos, self.window_start_size, self.window_start_pos) {
            
            let delta_x = cursor_x - start_pos.0;
            let delta_y = cursor_y - start_pos.1;
            
            let mut new_width = start_size.0 as i32;
            let mut new_height = start_size.1 as i32;
            let mut new_x = window_pos.0;
            let mut new_y = window_pos.1;
            
            // Calculate new dimensions based on edge
            match edge {
                ResizeEdge::Right => {
                    new_width += delta_x as i32;
                }
                ResizeEdge::Left => {
                    new_width -= delta_x as i32;
                    new_x += delta_x as i32;
                }
                ResizeEdge::Bottom => {
                    new_height += delta_y as i32;
                }
                ResizeEdge::Top => {
                    new_height -= delta_y as i32;
                    new_y += delta_y as i32;
                }
                ResizeEdge::TopLeft => {
                    new_width -= delta_x as i32;
                    new_height -= delta_y as i32;
                    new_x += delta_x as i32;
                    new_y += delta_y as i32;
                }
                ResizeEdge::TopRight => {
                    new_width += delta_x as i32;
                    new_height -= delta_y as i32;
                    new_y += delta_y as i32;
                }
                ResizeEdge::BottomLeft => {
                    new_width -= delta_x as i32;
                    new_height += delta_y as i32;
                    new_x += delta_x as i32;
                }
                ResizeEdge::BottomRight => {
                    new_width += delta_x as i32;
                    new_height += delta_y as i32;
                }
            }
            
            // Apply minimum size constraints
            const MIN_WIDTH: i32 = 400;
            const MIN_HEIGHT: i32 = 300;
            
            new_width = new_width.max(MIN_WIDTH);
            new_height = new_height.max(MIN_HEIGHT);
            
            // Apply the new size
            let _ = window.set_inner_size(LogicalSize::new(new_width as f64, new_height as f64));
            
            // Move window if resizing from left or top edges
            if matches!(edge, ResizeEdge::Left | ResizeEdge::Top | ResizeEdge::TopLeft | ResizeEdge::TopRight | ResizeEdge::BottomLeft) {
                let _ = window.set_outer_position(LogicalPosition::new(new_x as f64, new_y as f64));
            }
        }
    }
    
    /// Check if currently resizing
    pub fn is_resizing(&self) -> bool {
        self.is_resizing
    }
    
    /// Get current resize edge
    pub fn current_edge(&self) -> Option<ResizeEdge> {
        self.current_edge
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_resize_handler_creation() {
        let handler = ResizeHandler::new(10);
        assert!(!handler.is_resizing());
        assert!(handler.current_edge().is_none());
    }
    
    #[test]
    fn test_hit_testing() {
        let handler = ResizeHandler::new(10);
        let metrics = WindowMetrics {
            width: 800,
            height: 600,
            titlebar_height: 32,
            control_size: 46,
            scale_factor: 1.0,
        };
        
        // Test corners
        assert_eq!(handler.hit_test(5.0, 40.0, &metrics), Some(ResizeEdge::Left));
        assert_eq!(handler.hit_test(795.0, 40.0, &metrics), Some(ResizeEdge::Right));
        assert_eq!(handler.hit_test(400.0, 5.0, &metrics), Some(ResizeEdge::Top));
        assert_eq!(handler.hit_test(400.0, 595.0, &metrics), Some(ResizeEdge::Bottom));
        
        // Test center (no edge)
        assert_eq!(handler.hit_test(400.0, 300.0, &metrics), None);
        
        // Test titlebar area (no resize allowed)
        assert_eq!(handler.hit_test(5.0, 20.0, &metrics), None);
    }
    
    #[test]
    fn test_edge_to_cursor_mapping() {
        assert_eq!(ResizeEdge::Top.to_cursor_icon(), CursorIcon::NsResize);
        assert_eq!(ResizeEdge::Bottom.to_cursor_icon(), CursorIcon::NsResize);
        assert_eq!(ResizeEdge::Left.to_cursor_icon(), CursorIcon::EwResize);
        assert_eq!(ResizeEdge::Right.to_cursor_icon(), CursorIcon::EwResize);
        assert_eq!(ResizeEdge::TopLeft.to_cursor_icon(), CursorIcon::NwseResize);
        assert_eq!(ResizeEdge::BottomRight.to_cursor_icon(), CursorIcon::NwseResize);
        assert_eq!(ResizeEdge::TopRight.to_cursor_icon(), CursorIcon::NeswResize);
        assert_eq!(ResizeEdge::BottomLeft.to_cursor_icon(), CursorIcon::NeswResize);
    }
}