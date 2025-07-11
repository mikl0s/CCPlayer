// window_frame.rs
// Demonstrates a borderless, resizable window

use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn main() {
    let event_loop = EventLoop::new();
    let _window = WindowBuilder::new()
        .with_title("Borderless Resizable Window")
        .with_decorations(false)
        .with_resizable(true)
        .with_inner_size(LogicalSize::new(800.0, 600.0))
        .build(&event_loop)
        .unwrap();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        if let Event::WindowEvent { event, .. } = event {
            match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                _ => (),
            }
        }
    });
