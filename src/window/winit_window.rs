//! Winit window implementation for CCPlayer
//! 
//! This module provides a borderless window with Alt+drag functionality
//! and edge resize handling using the winit crate.

use crate::utils::error::{CCPlayerError, Result, IntoPlayerError};
use crate::utils::config::WindowConfig;
use crate::window::{Window, WindowEvent, WindowMetrics};
use winit::{
    dpi::{LogicalSize, PhysicalPosition},
    event_loop::{EventLoop, EventLoopBuilder},
    window::{Window as WinitWindow, WindowBuilder, CursorIcon},
};
use std::sync::{Arc, Mutex};

pub mod events;
pub mod drag; 
pub mod resize;

use self::events::EventHandler;
use self::drag::DragHandler;
use self::resize::{ResizeHandler, ResizeEdge};

/// Winit-based window implementation
pub struct WinitWindowImpl {
    /// The underlying winit window
    window: Arc<WinitWindow>,
    
    /// Event loop for the window
    event_loop: Option<EventLoop<()>>,
    
    /// Window metrics
    metrics: WindowMetrics,
    
    /// Event handler
    event_handler: EventHandler,
    
    /// Drag handler for Alt+drag functionality
    drag_handler: DragHandler,
    
    /// Resize handler for edge detection and resizing
    resize_handler: ResizeHandler,
    
    /// Window visibility state
    visible: bool,
    
    /// Fullscreen state
    fullscreen: bool,
}

impl Window for WinitWindowImpl {
    fn new(config: WindowConfig) -> Result<Self> 
    where 
        Self: Sized 
    {
        // Create event loop
        let event_loop = EventLoopBuilder::new()
            .build()
            .window_err("Failed to create event loop")?;
        
        // Create window
        let window = WindowBuilder::new()
            .with_title(&config.title)
            .with_decorations(false)  // Borderless window
            .with_resizable(true)
            .with_inner_size(LogicalSize::new(config.width as f64, config.height as f64))
            .with_always_on_top(config.always_on_top)
            .with_visible(!config.start_minimized)
            .build(&event_loop)
            .window_err("Failed to create window")?;
        
        let window = Arc::new(window);
        let scale_factor = window.scale_factor();
        
        // Initialize window metrics
        let metrics = WindowMetrics {
            width: config.width,
            height: config.height,
            titlebar_height: 32,
            control_size: 46,
            scale_factor,
        };
        
        // Create handlers
        let event_handler = EventHandler::new();
        let drag_handler = DragHandler::new();
        let resize_handler = ResizeHandler::new(10); // 10px edge detection border
        
        Ok(Self {
            window,
            event_loop: Some(event_loop),
            metrics,
            event_handler,
            drag_handler,
            resize_handler,
            visible: !config.start_minimized,
            fullscreen: config.fullscreen,
        })
    }
    
    fn show(&mut self) -> Result<()> {
        self.window.set_visible(true);
        self.visible = true;
        Ok(())
    }
    
    fn hide(&mut self) -> Result<()> {
        self.window.set_visible(false);
        self.visible = false;
        Ok(())
    }
    
    fn set_title(&mut self, title: &str) -> Result<()> {
        self.window.set_title(title);
        Ok(())
    }
    
    fn set_fullscreen(&mut self, fullscreen: bool) -> Result<()> {
        use winit::window::Fullscreen;
        
        if fullscreen {
            let monitor = self.window.current_monitor()
                .or_else(|| self.window.primary_monitor())
                .ok_or_else(|| CCPlayerError::Window("No monitor found".to_string()))?;
            
            self.window.set_fullscreen(Some(Fullscreen::Borderless(Some(monitor))));
        } else {
            self.window.set_fullscreen(None);
        }
        
        self.fullscreen = fullscreen;
        Ok(())
    }
    
    fn is_fullscreen(&self) -> bool {
        self.fullscreen
    }
    
    fn handle_events(&mut self) -> Result<Vec<WindowEvent>> {
        // This will be called from the main event loop
        // For now, return empty vec as events will be processed in run()
        Ok(vec![])
    }
    
    fn handle(&self) -> &dyn std::any::Any {
        &self.window
    }
}

impl WinitWindowImpl {
    /// Run the window event loop
    pub fn run(mut self) -> Result<()> {
        let window = self.window.clone();
        let mut events = Vec::new();
        
        // Take ownership of the event loop
        let event_loop = self.event_loop.take()
            .ok_or_else(|| CCPlayerError::Window("Event loop already consumed".to_string()))?;
        
        // Run the event loop
        event_loop.run(move |event, elwt| {
            use winit::event::{Event, WindowEvent as WinitWindowEvent};
            use winit::event_loop::ControlFlow;
            
            match event {
                Event::WindowEvent { event: win_event, .. } => {
                    // Handle drag events
                    if let Some(drag_event) = self.drag_handler.handle_event(&win_event, &window) {
                        events.push(drag_event);
                    }
                    
                    // Handle resize events
                    if let Some((resize_event, cursor)) = self.resize_handler.handle_event(&win_event, &window, &self.metrics) {
                        // Update cursor based on resize edge
                        window.set_cursor_icon(cursor);
                        events.push(resize_event);
                    }
                    
                    // Handle general window events
                    if let Some(window_event) = self.event_handler.handle_event(win_event, &self.metrics) {
                        events.push(window_event);
                        
                        // Handle special events
                        match &window_event {
                            WindowEvent::CloseRequested => {
                                elwt.exit();
                            }
                            WindowEvent::Resized { width, height } => {
                                self.metrics.width = *width;
                                self.metrics.height = *height;
                            }
                            _ => {}
                        }
                    }
                }
                Event::AboutToWait => {
                    // Process any pending events here if needed
                    events.clear();
                }
                _ => {}
            }
        }).window_err("Event loop error")?;
        
        Ok(())
    }
    
    /// Get the winit window handle
    pub fn winit_window(&self) -> &WinitWindow {
        &self.window
    }
    
    /// Update window metrics (for DPI changes, etc.)
    pub fn update_metrics(&mut self) {
        let size = self.window.inner_size();
        self.metrics.width = size.width;
        self.metrics.height = size.height;
        self.metrics.scale_factor = self.window.scale_factor();
    }
}

#[cfg(target_os = "windows")]
impl WinitWindowImpl {
    /// Windows-specific hit testing for resize regions
    pub fn hit_test(&self, x: f64, y: f64) -> Option<ResizeEdge> {
        self.resize_handler.hit_test(x, y, &self.metrics)
    }
}


