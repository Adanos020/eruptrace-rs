use eruptrace_rs::shaders::cs;
use eruptrace_rs::vulkan_context::VulkanContext;

use std::sync::Arc;

use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage};
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::format::Format;
use vulkano::image::view::ImageView;
use vulkano::image::{ImageDimensions, StorageImage};
use vulkano::instance::{Instance, InstanceExtensions};
use vulkano::pipeline::{ComputePipeline, Pipeline, PipelineBindPoint};
use vulkano::sync::GpuFuture;
use vulkano::Version;

use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

fn main() {
    let instance = Instance::new(None, Version::V1_5, &InstanceExtensions::none(), None)
        .expect("Cannot create Vulkan instance.");
    let mut vk_context = VulkanContext::new(&instance);
    let queue = vk_context
        .queues
        .next()
        .expect("Cannot pull the first queue.");

    let image = StorageImage::new(
        Arc::clone(&vk_context.device),
        ImageDimensions::Dim2d {
            width: 1024,
            height: 1024,
            array_layers: 1,
        },
        Format::R8G8B8A8_UNORM,
        Some(vk_context.queue_family),
    )
    .expect("Cannot create image.");
    let image_view = ImageView::new(Arc::clone(&image)).expect("Cannot create image view.");

    let out_buf = {
        let out_iter = (0..1024 * 1024 * 4).map(|_| 0u8);
        CpuAccessibleBuffer::from_iter(
            Arc::clone(&vk_context.device),
            BufferUsage::all(),
            false,
            out_iter,
        )
        .expect("Cannot create output buffer.")
    };

    let shader = cs::load(Arc::clone(&vk_context.device)).expect("Cannot load shader.");
    let pipeline = ComputePipeline::new(
        Arc::clone(&vk_context.device),
        shader
            .entry_point("main")
            .expect("Cannot bind shader with entry point 'main'."),
        &(),
        None,
        |_| {},
    )
    .expect("Cannot create compute pipeline.");

    let dset = {
        let layout = pipeline
            .layout()
            .descriptor_set_layouts()
            .get(0)
            .expect("Could not get descriptor set #0.");
        let image_view = Arc::clone(&image_view);
        PersistentDescriptorSet::new(
            Arc::clone(layout),
            [WriteDescriptorSet::image_view(0, image_view)],
        )
        .expect("Cannot build descriptor set.")
    };

    let cb = {
        let mut cb_builder = AutoCommandBufferBuilder::primary(
            Arc::clone(&vk_context.device),
            vk_context.queue_family,
            CommandBufferUsage::OneTimeSubmit,
        )
        .expect("Cannot create command buffer builder");
        let image = Arc::clone(&image);
        let out_buf = Arc::clone(&out_buf);
        cb_builder
            .bind_pipeline_compute(Arc::clone(&pipeline))
            .bind_descriptor_sets(
                PipelineBindPoint::Compute,
                Arc::clone(pipeline.layout()),
                0,
                dset,
            )
            .dispatch([1024 / 8, 1024 / 8, 1])
            .expect("Cannot dispatch compute pipeline.")
            .copy_image_to_buffer(image, out_buf)
            .expect("Cannot copy image to output buffer.");
        cb_builder.build().expect("Cannot create command buffer.")
    };

    vulkano::sync::now(Arc::clone(&vk_context.device))
        .then_execute(Arc::clone(&queue), cb)
        .expect("Could not execute commands.")
        .then_signal_fence_and_flush()
        .expect("Could not finalise commands.")
        .wait(None)
        .expect("Could not wait for command execution.");

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
