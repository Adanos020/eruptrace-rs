use crate::shaders::shaders_image;
use crate::vulkan_context::VulkanContext;

use std::sync::Arc;

use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer, SubpassContents,
};
use vulkano::format::Format;
use vulkano::image::view::ImageView;
use vulkano::image::{ImageDimensions, StorageImage};
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::vertex_input::BuffersDefinition;
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::GraphicsPipeline;
use vulkano::render_pass::{Framebuffer, Subpass};

#[derive(Default, Debug, Copy, Clone)]
pub struct Vertex {
    position: [f32; 2],
}

vulkano::impl_vertex!(Vertex, position);

pub fn render_image(
    vk_context: &VulkanContext,
    (width, height): (u32, u32),
    image: Arc<StorageImage>,
) -> PrimaryAutoCommandBuffer {
    let vertices = [
        Vertex {
            position: [-0.50, -0.50],
        },
        Vertex {
            position: [0.00, 0.50],
        },
        Vertex {
            position: [0.50, -0.25],
        },
    ];
    let vertex_buffer = CpuAccessibleBuffer::from_data(
        Arc::clone(&vk_context.device),
        BufferUsage::vertex_buffer(),
        false,
        vertices,
    )
    .expect("Cannot create vertex buffer.");

    let vertex_shader = shaders_image::load_vertex(Arc::clone(&vk_context.device))
        .expect("Cannot load vertex shader.");
    let fragment_shader = shaders_image::load_fragment(Arc::clone(&vk_context.device))
        .expect("Cannot load fragment shader.");

    let render_pass = vulkano::single_pass_renderpass!(
        Arc::clone(&vk_context.device),
        attachments: {
            color: {
                load: Clear,
                store: Store,
                format: Format::R8G8B8A8_UNORM,
                samples: 1,
            }
        },
        pass: {
            color: [color],
            depth_stencil: {}
        }
    )
    .expect("Cannot create render pass.");

    let frame_image = StorageImage::new(
        Arc::clone(&vk_context.device),
        ImageDimensions::Dim2d {
            width,
            height,
            array_layers: 1,
        },
        Format::R8G8B8A8_UNORM,
        Some(vk_context.queue_family),
    )
    .expect("Cannot create frame image.");
    let frame_view = ImageView::new(frame_image).expect("Cannot create frame view.");
    let framebuffer = Framebuffer::start(Arc::clone(&render_pass))
        .add(frame_view)
        .expect("Cannot add frame view to framebuffer.")
        .build()
        .expect("Cannot create framebuffer.");

    let viewport = Viewport {
        origin: [0.0, 0.0],
        dimensions: [width as f32, height as f32],
        depth_range: 0.0..1.0,
    };

    let pipeline = GraphicsPipeline::start()
        .vertex_input_state(BuffersDefinition::new().vertex::<Vertex>())
        .vertex_shader(
            vertex_shader
                .entry_point("main")
                .expect("Cannot bind shader with entry point 'main'."),
            (),
        )
        .input_assembly_state(InputAssemblyState::new())
        .viewport_state(ViewportState::viewport_fixed_scissor_irrelevant([viewport]))
        .fragment_shader(
            fragment_shader
                .entry_point("main")
                .expect("Cannot bind shader with entry point 'main'."),
            (),
        )
        .render_pass(
            Subpass::from(Arc::clone(&render_pass), 0)
                .expect("Cannot create subpass from render_pass."),
        )
        .build(Arc::clone(&vk_context.device))
        .expect("Cannot create graphics pipeline.");

    let mut cb_builder = AutoCommandBufferBuilder::primary(
        Arc::clone(&vk_context.device),
        vk_context.queue_family,
        CommandBufferUsage::OneTimeSubmit,
    )
    .expect("Cannot create command buffer builder.");

    cb_builder
        .begin_render_pass(
            framebuffer,
            SubpassContents::Inline,
            vec![[0.0, 0.0, 0.0, 1.0].into()],
        )
        .expect("Cannot begin render pass.")
        .bind_pipeline_graphics(pipeline)
        .bind_vertex_buffers(0, vertex_buffer)
        .draw(vertices.len() as u32, 1, 0, 0)
        .expect("Cannot execute draw command.")
        .end_render_pass()
        .expect("Cannot end render pass.");
    cb_builder.build().expect("Cannot create command buffer.")
}
