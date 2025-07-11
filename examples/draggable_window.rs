// draggable_window.rs
// Demonstrates Alt + Left Mouse dragging for moving a borderless window using winit

use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, MouseButton, MouseScrollDelta, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{WindowBuilder, Window},
};

fn main() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Draggable Window")
        .with_decorations(false)
        .with_inner_size(LogicalSize::new(800.0, 600.0))
        .build(&event_loop)
        .unwrap();

    let mut is_dragging = false;
    let mut alt_held = false;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::KeyboardInput { input, .. } => {
                    if let Some(key) = input.virtual_keycode {
                        if key == VirtualKeyCode::LAlt {
                            alt_held = input.state == ElementState::Pressed;
                        }
                    }
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    if button == MouseButton::Left && alt_held {
                        is_dragging = state == ElementState::Pressed;
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    if is_dragging {
                        let _ = window.set_outer_position(position.to_logical(window.scale_factor()));
                    }
                }
                _ => (),
            },
            _ => (),
        }
    });
