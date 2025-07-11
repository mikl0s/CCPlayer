//! Integration tests for window functionality
//!
//! These tests verify:
//! - Window creation and management
//! - Event handling
//! - Drag functionality
//! - Resize operations
//! - Window controls (minimize, maximize, close)

use anyhow::Result;
use ccplayer::window::{WindowBuilder, WindowEvent, DragState};
use ccplayer_integration_tests::mock_events;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_window_creation() -> Result<()> {
    // Create a window with custom configuration
    let window = WindowBuilder::new()
        .with_title("Test Window")
        .with_inner_size(1280, 720)
        .with_decorations(false)
        .with_resizable(true)
        .build()?;
    
    assert_eq!(window.title(), "Test Window");
    assert_eq!(window.inner_size(), (1280, 720));
    assert!(!window.is_decorated());
    assert!(window.is_resizable());
    
    Ok(())
}

#[tokio::test]
async fn test_window_show_hide() -> Result<()> {
    let mut window = WindowBuilder::new()
        .with_title("Show/Hide Test")
        .build()?;
    
    // Window should be visible by default
    assert!(window.is_visible());
    
    // Hide window
    window.hide()?;
    assert!(!window.is_visible());
    
    // Show window again
    window.show()?;
    assert!(window.is_visible());
    
    Ok(())
}

#[tokio::test]
async fn test_window_events() -> Result<()> {
    let mut window = WindowBuilder::new()
        .with_title("Event Test")
        .build()?;
    
    // Simulate and handle events
    let test_events = mock_events::generate_event_sequence();
    let mut received_events = Vec::new();
    
    for event in test_events {
        window.inject_event(event.clone())?;
        
        // Process events
        while let Some(event) = window.poll_event() {
            received_events.push(event);
        }
    }
    
    // Verify we received the expected events
    assert!(!received_events.is_empty());
    
    // Check for specific event types
    let has_resize = received_events.iter().any(|e| matches!(e, WindowEvent::Resized { .. }));
    let has_mouse = received_events.iter().any(|e| matches!(e, WindowEvent::MouseInput { .. }));
    let has_keyboard = received_events.iter().any(|e| matches!(e, WindowEvent::KeyboardInput { .. }));
    
    assert!(has_resize, "No resize event received");
    assert!(has_mouse, "No mouse event received");
    assert!(has_keyboard, "No keyboard event received");
    
    Ok(())
}

#[tokio::test]
async fn test_window_drag() -> Result<()> {
    let mut window = WindowBuilder::new()
        .with_title("Drag Test")
        .with_decorations(false)
        .build()?;
    
    let initial_position = window.outer_position();
    
    // Simulate drag sequence
    let drag_events = mock_events::generate_drag_sequence();
    
    for event in drag_events {
        window.inject_event(event)?;
        window.update_drag_state()?;
    }
    
    // Window position should have changed
    let final_position = window.outer_position();
    assert_ne!(initial_position, final_position);
    
    Ok(())
}

#[tokio::test]
async fn test_window_resize() -> Result<()> {
    let mut window = WindowBuilder::new()
        .with_title("Resize Test")
        .with_inner_size(800, 600)
        .with_resizable(true)
        .build()?;
    
    let initial_size = window.inner_size();
    
    // Test programmatic resize
    window.set_inner_size(1024, 768)?;
    let new_size = window.inner_size();
    
    assert_eq!(new_size, (1024, 768));
    assert_ne!(initial_size, new_size);
    
    // Test resize constraints
    window.set_min_inner_size(Some((640, 480)))?;
    window.set_max_inner_size(Some((1920, 1080)))?;
    
    // Try to resize below minimum
    window.set_inner_size(320, 240)?;
    assert_eq!(window.inner_size(), (640, 480)); // Should be clamped to minimum
    
    // Try to resize above maximum
    window.set_inner_size(2560, 1440)?;
    assert_eq!(window.inner_size(), (1920, 1080)); // Should be clamped to maximum
    
    Ok(())
}

#[tokio::test]
async fn test_window_fullscreen() -> Result<()> {
    let mut window = WindowBuilder::new()
        .with_title("Fullscreen Test")
        .build()?;
    
    assert!(!window.is_fullscreen());
    
    // Enter fullscreen
    window.set_fullscreen(true)?;
    assert!(window.is_fullscreen());
    
    // Exit fullscreen
    window.set_fullscreen(false)?;
    assert!(!window.is_fullscreen());
    
    Ok(())
}

#[tokio::test]
async fn test_window_minimize_maximize() -> Result<()> {
    let mut window = WindowBuilder::new()
        .with_title("Min/Max Test")
        .build()?;
    
    // Test minimize
    window.minimize()?;
    assert!(window.is_minimized());
    
    window.restore()?;
    assert!(!window.is_minimized());
    
    // Test maximize
    window.maximize()?;
    assert!(window.is_maximized());
    
    window.restore()?;
    assert!(!window.is_maximized());
    
    Ok(())
}

#[tokio::test]
async fn test_window_focus() -> Result<()> {
    let mut window = WindowBuilder::new()
        .with_title("Focus Test")
        .build()?;
    
    // Request focus
    window.focus()?;
    
    // Note: We can't reliably test if the window actually has focus
    // as it depends on the window manager and other system factors
    
    Ok(())
}

#[tokio::test]
async fn test_window_title_updates() -> Result<()> {
    let mut window = WindowBuilder::new()
        .with_title("Initial Title")
        .build()?;
    
    assert_eq!(window.title(), "Initial Title");
    
    // Update title
    window.set_title("Updated Title")?;
    assert_eq!(window.title(), "Updated Title");
    
    // Test with empty title
    window.set_title("")?;
    assert_eq!(window.title(), "");
    
    Ok(())
}

#[tokio::test]
async fn test_window_cursor() -> Result<()> {
    let mut window = WindowBuilder::new()
        .with_title("Cursor Test")
        .build()?;
    
    // Test cursor visibility
    window.set_cursor_visible(false)?;
    assert!(!window.is_cursor_visible());
    
    window.set_cursor_visible(true)?;
    assert!(window.is_cursor_visible());
    
    // Test cursor grab
    window.set_cursor_grab(true)?;
    assert!(window.is_cursor_grabbed());
    
    window.set_cursor_grab(false)?;
    assert!(!window.is_cursor_grabbed());
    
    Ok(())
}

#[tokio::test]
async fn test_window_always_on_top() -> Result<()> {
    let mut window = WindowBuilder::new()
        .with_title("Always on Top Test")
        .with_always_on_top(true)
        .build()?;
    
    assert!(window.is_always_on_top());
    
    window.set_always_on_top(false)?;
    assert!(!window.is_always_on_top());
    
    window.set_always_on_top(true)?;
    assert!(window.is_always_on_top());
    
    Ok(())
}

#[tokio::test]
async fn test_window_opacity() -> Result<()> {
    let mut window = WindowBuilder::new()
        .with_title("Opacity Test")
        .build()?;
    
    // Default opacity should be 1.0
    assert!((window.opacity() - 1.0).abs() < 0.01);
    
    // Set semi-transparent
    window.set_opacity(0.8)?;
    assert!((window.opacity() - 0.8).abs() < 0.01);
    
    // Test boundaries
    window.set_opacity(0.0)?;
    assert_eq!(window.opacity(), 0.0);
    
    window.set_opacity(1.0)?;
    assert_eq!(window.opacity(), 1.0);
    
    // Test clamping
    window.set_opacity(1.5)?;
    assert_eq!(window.opacity(), 1.0);
    
    window.set_opacity(-0.5)?;
    assert_eq!(window.opacity(), 0.0);
    
    Ok(())
}

#[tokio::test]
async fn test_multiple_windows() -> Result<()> {
    // Create multiple windows
    let mut windows = Vec::new();
    
    for i in 0..3 {
        let window = WindowBuilder::new()
            .with_title(&format!("Window {}", i))
            .with_inner_size(800, 600)
            .build()?;
        
        windows.push(window);
    }
    
    // Verify all windows exist and have correct titles
    for (i, window) in windows.iter().enumerate() {
        assert_eq!(window.title(), format!("Window {}", i));
    }
    
    Ok(())
}

#[tokio::test]
#[cfg_attr(not(feature = "stress-tests"), ignore)]
async fn test_window_stress() -> Result<()> {
    let mut window = WindowBuilder::new()
        .with_title("Stress Test")
        .build()?;
    
    // Rapid state changes
    for _ in 0..100 {
        window.show()?;
        window.hide()?;
    }
    
    // Rapid resize
    for i in 0..50 {
        let size = 600 + (i * 10);
        window.set_inner_size(size, size)?;
    }
    
    // Rapid title updates
    for i in 0..100 {
        window.set_title(&format!("Title {}", i))?;
    }
    
    // Window should still be functional
    window.show()?;
    assert!(window.is_visible());
    
    Ok(())
}