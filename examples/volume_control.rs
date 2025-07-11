// volume_control.rs
// Adjusts volume using mouse scroll

use winit::{
    event::{Event, WindowEvent, MouseScrollDelta},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn main() {
    let event_loop = EventLoop::new();
    let _window = WindowBuilder::new()
        .with_title("Scroll to Change Volume")
        .build(&event_loop)
        .unwrap();

    let mut volume: i32 = 50;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::MouseWheel { delta, .. } => {
                    let scroll = match delta {
                        MouseScrollDelta::LineDelta(_, y) => y,
                        MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
                    };
                    volume = (volume as f32 + scroll).clamp(0.0, 100.0) as i32;
                    println!("Volume: {}%", volume);
                }
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                _ => (),
            },
            _ => (),
        }
    });
