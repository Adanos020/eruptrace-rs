use crate::{camera::CameraUniformBuffer, rt_shaders, scene::SceneBuffers};
use std::sync::Arc;
use vulkano::{
    buffer::{BufferUsage, ImmutableBuffer, TypedBufferAccess},
    command_buffer::{AutoCommandBufferBuilder, CommandBufferExecFuture, PrimaryAutoCommandBuffer},
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
    sync::NowFuture,
};

#[derive(Copy, Clone, Default, Debug)]
struct RenderSurfaceVertex {
    pub position: [f32; 2],
}

vulkano::impl_vertex!(RenderSurfaceVertex, position);

#[derive(Clone)]
pub struct RenderSurface {
    vertex_buffer: Arc<ImmutableBuffer<[RenderSurfaceVertex]>>,
    pipeline: Arc<GraphicsPipeline>,
    uniform_descriptor_set: Arc<PersistentDescriptorSet>,
}

pub type RenderSurfaceVbFuture = CommandBufferExecFuture<NowFuture, PrimaryAutoCommandBuffer>;

impl RenderSurface {
    pub fn new(
        queue: Arc<Queue>,
        render_pass: Arc<RenderPass>,
        camera_buffer: CameraUniformBuffer,
        scene_buffers: SceneBuffers,
    ) -> (Self, RenderSurfaceVbFuture) {
        let vertices = [
            RenderSurfaceVertex {
                position: [-1.0, -1.0],
            },
            RenderSurfaceVertex {
                position: [1.0, -1.0],
            },
            RenderSurfaceVertex {
                position: [-1.0, 1.0],
            },
            RenderSurfaceVertex {
                position: [1.0, 1.0],
            },
        ];

        let (vertex_buffer, vb_future) = {
            ImmutableBuffer::from_iter(vertices, BufferUsage::vertex_buffer(), queue.clone())
                .expect("Cannot create vertex buffer.")
        };

        let pipeline = {
            let vertex_shader = rt_shaders::load_vertex(queue.device().clone())
                .expect("Cannot load vertex shader.");
            let fragment_shader = rt_shaders::load_fragment(queue.device().clone())
                .expect("Cannot load fragment shader.");
            GraphicsPipeline::start()
                .vertex_input_state(BuffersDefinition::new().vertex::<RenderSurfaceVertex>())
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
            let textures_image_view = ImageView::start(scene_buffers.textures_image)
                .ty(ImageViewType::Dim2dArray)
                .build()
                .expect("Cannot create textures image.");
            let normal_maps_image_view = ImageView::start(scene_buffers.normal_maps_image)
                .ty(ImageViewType::Dim2dArray)
                .build()
                .expect("Cannot create normal maps image.");

            let textures_sampler = Sampler::start(queue.device().clone())
                .filter(Filter::Linear)
                .address_mode(SamplerAddressMode::Repeat)
                .build()
                .expect("Cannot build sampler for textures.");
            let normal_maps_sampler = Sampler::start(queue.device().clone())
                .filter(Filter::Linear)
                .address_mode(SamplerAddressMode::Repeat)
                .build()
                .expect("Cannot build sampler for normal maps.");

            let layout = pipeline
                .layout()
                .descriptor_set_layouts()
                .get(0)
                .expect("Cannot get the layout of descriptor set 0.");
            PersistentDescriptorSet::new(
                layout.clone(),
                [
                    WriteDescriptorSet::image_view_sampler(
                        0,
                        textures_image_view,
                        textures_sampler,
                    ),
                    WriteDescriptorSet::image_view_sampler(
                        1,
                        normal_maps_image_view,
                        normal_maps_sampler,
                    ),
                    WriteDescriptorSet::buffer(2, camera_buffer),
                    WriteDescriptorSet::buffer(3, scene_buffers.materials_buffer),
                    WriteDescriptorSet::buffer(4, scene_buffers.shapes_buffer),
                    WriteDescriptorSet::buffer(5, scene_buffers.mesh_metas_buffer),
                    WriteDescriptorSet::buffer(6, scene_buffers.mesh_data_buffer),
                ],
            )
            .expect("Cannot create descriptor set.")
        };

        (
            Self {
                vertex_buffer,
                pipeline,
                uniform_descriptor_set,
            },
            vb_future,
        )
    }

    pub fn draw(&self, cb_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>) {
        cb_builder
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.pipeline.layout().clone(),
                0,
                self.uniform_descriptor_set.clone(),
            )
            .bind_vertex_buffers(0, self.vertex_buffer.clone())
            .draw(self.vertex_buffer.len() as u32, 1, 0, 0)
            .expect("Cannot execute draw command.");
    }
}
