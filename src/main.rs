use eruptrace_rs::vulkan_context::VulkanContext;

use vulkano::instance::{Instance, InstanceExtensions};
use vulkano::Version;

use eruptrace_rs::trace::render_scene;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

fn main() {
    let instance = Instance::new(None, Version::V1_5, &InstanceExtensions::none(), None)
        .expect("Cannot create Vulkan instance.");
    let vk_context = VulkanContext::new(&instance);

    let (out_image, out_buffer) = render_scene(&vk_context, (1024, 1024));

    let event_loop = EventLoop::new();
    let _window = WindowBuilder::new()
        .with_title("ErupTrace")
        .build(&event_loop)
        .expect("Cannot create window.");

    event_loop.run(|event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::MainEventsCleared => {
                // TODO: Render here
            }
            _ => {}
        }
    });
}
