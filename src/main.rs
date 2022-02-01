use winit::{
    event_loop::EventLoop,
    window::WindowBuilder,
};
use winit::event::{Event, WindowEvent};
use winit::event_loop::ControlFlow;

fn main() {
    let event_loop = EventLoop::new();
    let _window = WindowBuilder::new()
        .with_title("ErupTrace")
        .build(&event_loop)
        .expect("Could not create window.");

    event_loop.run(|event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::MainEventsCleared => {
                // TODO: Render here
            }
            _ => {}
        }
    });
}