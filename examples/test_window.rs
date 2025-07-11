//! Test the winit window implementation
//! 
//! This example demonstrates the borderless window with Alt+drag
//! and edge resize functionality.

use ccplayer::window::{Window, WinitWindowImpl, WindowEvent};
use ccplayer::utils::config::WindowConfig;

fn main() -> anyhow::Result<()> {
    // Initialize logger
    env_logger::init();
    
    // Create window configuration
    let config = WindowConfig {
        width: 1024,
        height: 768,
        fullscreen: false,
        title: "CCPlayer Window Test".to_string(),
        always_on_top: false,
        start_minimized: false,
    };
    
    // Create the window
    println!("Creating borderless window...");
    println!("Controls:");
    println!("  - Alt + Left Mouse: Drag window");
    println!("  - Mouse near edges: Resize window");
    println!("  - Mouse wheel: Volume control (logged to console)");
    println!("  - Escape: Exit");
    
    let mut window = WinitWindowImpl::new(config)?;
    window.show()?;
    
    // Run the window (this will block until the window is closed)
    window.run()?;
    
    Ok(())
}