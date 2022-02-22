use crate::{
    camera::CameraUniformBuffer,
    rt_shaders,
    scene::{MaterialsBuffer, ShapesBuffer, TexturesImage},
};
use std::sync::Arc;
use vulkano::{
    buffer::{BufferUsage, ImmutableBuffer, TypedBufferAccess},
    command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer},
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    device::Queue,
    image::view::{ImageView, ImageViewType},
    pipeline::{
        graphics::{
            input_assembly::{InputAssemblyState, PrimitiveTopology},
            vertex_input::BuffersDefinition,
            viewport::ViewportState,
        },
        {GraphicsPipeline, Pipeline, PipelineBindPoint},
    },
    render_pass::{RenderPass, Subpass},
    sampler::{Filter, Sampler, SamplerAddressMode},
    sync::GpuFuture,
};

#[derive(Copy, Clone, Default, Debug)]
pub struct Vertex {
    pub position: [f32; 2],
}

vulkano::impl_vertex!(Vertex, position);

pub struct RenderSurface {
    pub vertex_buffer: Arc<ImmutableBuffer<[Vertex]>>,
    pub pipeline: Arc<GraphicsPipeline>,
    pub uniform_descriptor_set: Arc<PersistentDescriptorSet>,
}

impl RenderSurface {
    pub fn new(
        queue: Arc<Queue>,
        render_pass: Arc<RenderPass>,
        camera_buf: CameraUniformBuffer,
        shapes_buf: ShapesBuffer,
        materials_buf: MaterialsBuffer,
        textures_img: TexturesImage,
    ) -> Self {
        let vertices = [
            Vertex {
                position: [-1.0, -1.0],
            },
            Vertex {
                position: [1.0, -1.0],
            },
            Vertex {
                position: [-1.0, 1.0],
            },
            Vertex {
                position: [1.0, 1.0],
            },
        ];

        let (vertex_buffer, vb_future) =
            ImmutableBuffer::from_iter(vertices, BufferUsage::vertex_buffer(), queue.clone())
                .expect("Cannot create vertex buffer.");

        vulkano::sync::now(queue.device().clone())
            .join(vb_future)
            .then_signal_fence_and_flush()
            .expect("Cannot upload vertex and index buffers.")
            .wait(None)
            .expect("Cannot wait.");

        let pipeline = {
            let vertex_shader = rt_shaders::load_vertex(queue.device().clone())
                .expect("Cannot load vertex shader.");
            let fragment_shader = rt_shaders::load_fragment(queue.device().clone())
                .expect("Cannot load fragment shader.");
            GraphicsPipeline::start()
                .vertex_input_state(BuffersDefinition::new().vertex::<Vertex>())
                .vertex_shader(
                    vertex_shader
                        .entry_point("main")
                        .expect("Cannot bind vertex shader with entry point 'main'."),
                    (),
                )
                .input_assembly_state(
                    InputAssemblyState::new().topology(PrimitiveTopology::TriangleStrip),
                )
                .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
                .fragment_shader(
                    fragment_shader
                        .entry_point("main")
                        .expect("Cannot bind fragment shader with entry point 'main'."),
                    (),
                )
                .render_pass(
                    Subpass::from(render_pass, 0).expect("Cannot create subpass for render pass."),
                )
                .build(queue.device().clone())
                .expect("Cannot create graphics pipeline object.")
        };

        let uniform_descriptor_set = {
            let textures_img_view = ImageView::start(textures_img)
                .ty(ImageViewType::Dim2dArray)
                .build()
                .expect("Cannot create textures image.");
            let textures_sampler = Sampler::start(queue.device().clone())
                .filter(Filter::Linear)
                .address_mode(SamplerAddressMode::ClampToEdge)
                .build()
                .expect("Cannot build sampler for textures.");
            let layout = pipeline
                .layout()
                .descriptor_set_layouts()
                .get(0)
                .expect("Cannot get the layout of descriptor set 0.");
            PersistentDescriptorSet::new(
                layout.clone(),
                [
                    WriteDescriptorSet::buffer(0, camera_buf),
                    WriteDescriptorSet::buffer(1, shapes_buf),
                    WriteDescriptorSet::buffer(2, materials_buf),
                    WriteDescriptorSet::image_view_sampler(3, textures_img_view, textures_sampler),
                ],
            )
            .expect("Cannot create descriptor set.")
        };

        Self {
            vertex_buffer,
            pipeline,
            uniform_descriptor_set,
        }
    }

    pub fn draw(&self, cb_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>) {
        cb_builder
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                Arc::clone(self.pipeline.layout()),
                0,
                self.uniform_descriptor_set.clone(),
            )
            .bind_vertex_buffers(0, self.vertex_buffer.clone())
            .draw(self.vertex_buffer.len() as u32, 1, 0, 0)
            .expect("Cannot execute draw command.");
    }
}
