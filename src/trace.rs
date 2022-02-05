use crate::shaders::cs;
use crate::vulkan_context::VulkanContext;

use std::sync::Arc;

use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage};
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::format::Format;
use vulkano::image::view::ImageView;
use vulkano::image::{ImageDimensions, StorageImage};
use vulkano::pipeline::{ComputePipeline, Pipeline, PipelineBindPoint};
use vulkano::sync::GpuFuture;

pub fn render_scene(
    vk_context: &VulkanContext,
    (width, height): (u32, u32),
) -> (Arc<StorageImage>, Arc<CpuAccessibleBuffer<[u8]>>) {
    let image = StorageImage::new(
        Arc::clone(&vk_context.device),
        ImageDimensions::Dim2d {
            width,
            height,
            array_layers: 1,
        },
        Format::R8G8B8A8_UNORM,
        Some(vk_context.queue_family),
    )
    .expect("Cannot create image.");
    let image_view = ImageView::new(Arc::clone(&image)).expect("Cannot create image view.");

    let out_buf = {
        let out_iter = (0..width * height * 4).map(|_| 0u8);
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

    let descriptor_set = {
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
                descriptor_set,
            )
            .dispatch([width / 8, height / 8, 1])
            .expect("Cannot dispatch compute pipeline.")
            .copy_image_to_buffer(image, out_buf)
            .expect("Cannot copy image to output buffer.");
        cb_builder.build().expect("Cannot create command buffer.")
    };

    vulkano::sync::now(Arc::clone(&vk_context.device))
        .then_execute(Arc::clone(&vk_context.queues[0]), cb)
        .expect("Could not execute commands.")
        .then_signal_fence_and_flush()
        .expect("Could not finalise commands.")
        .wait(None)
        .expect("Could not wait for command execution.");

    (image, out_buf)
}
