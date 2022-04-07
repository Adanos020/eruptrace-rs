use crate::shaders::*;
use erupt::{vk, DeviceLoader};
use eruptrace_scene::{CameraUniform, SceneBuffers};
use eruptrace_vk::{
    contexts::{PipelineContext, RenderContext},
    pipeline::{
        ColorAttachmentInfo, DescriptorBindingCreateInfo, DescriptorSetCreateInfo,
        GraphicsPipeline, GraphicsPipelineCreateInfo, RasterisationStateInfo, SamplerCreateInfo,
    },
    AllocatedBuffer, VulkanContext,
};
use nalgebra_glm as glm;
use std::ffi::c_void;
use std140::repr_std140;
use vk_mem_erupt as vma;

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct Vertex {
    pub position: glm::Vec2,
}

#[repr_std140]
#[derive(Copy, Clone, Debug)]
struct PushConstants {
    n_triangles: std140::uint,
    use_bih: std140::boolean,
}

#[derive(Clone)]
pub struct RenderSurface {
    vertex_buffer: AllocatedBuffer<Vertex>,
    push_constants: PushConstants,
    graphics_pipeline: GraphicsPipeline,
}

impl RenderSurface {
    pub fn new(
        vk_ctx: VulkanContext,
        pipeline_ctx: PipelineContext,
        camera_buffer: &AllocatedBuffer<CameraUniform>,
        scene_buffers: &SceneBuffers,
    ) -> Self {
        let vertex_buffer = {
            let vertices = [
                Vertex {
                    position: glm::vec2(-1.0, -1.0),
                },
                Vertex {
                    position: glm::vec2(1.0, -1.0),
                },
                Vertex {
                    position: glm::vec2(-1.0, 1.0),
                },
                Vertex {
                    position: glm::vec2(1.0, 1.0),
                },
            ];

            let buffer_info = vk::BufferCreateInfoBuilder::new()
                .usage(vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);
            AllocatedBuffer::with_data(
                vk_ctx.allocator.clone(),
                &buffer_info,
                vma::MemoryUsage::CpuToGpu,
                &vertices,
            )
        };

        let graphics_pipeline = GraphicsPipeline::new(
            vk_ctx,
            GraphicsPipelineCreateInfo {
                vertex_shader: VERTEX_SHADER,
                fragment_shader: FRAGMENT_SHADER,
                color_attachment_infos: vec![ColorAttachmentInfo {
                    format: pipeline_ctx.surface_format.format,
                    color_write_mask: vk::ColorComponentFlags::all(),
                }],
                push_constant_ranges: vec![vk::PushConstantRangeBuilder::new()
                    .offset(0)
                    .size(std::mem::size_of::<PushConstants>() as u32)
                    .stage_flags(vk::ShaderStageFlags::FRAGMENT)],
                input_assembly: vk::PipelineInputAssemblyStateCreateInfoBuilder::new()
                    .topology(vk::PrimitiveTopology::TRIANGLE_STRIP)
                    .primitive_restart_enable(false),
                vertex_input_bindings: vec![vk::VertexInputBindingDescriptionBuilder::new()
                    .binding(0)
                    .input_rate(vk::VertexInputRate::VERTEX)
                    .stride(std::mem::size_of::<Vertex>() as u32)],
                vertex_input_attributes: vec![
                    // position
                    vk::VertexInputAttributeDescriptionBuilder::new()
                        .binding(0)
                        .location(0)
                        .format(vk::Format::R32G32_SFLOAT)
                        .offset(0),
                ],
                rasterisation_state: RasterisationStateInfo {
                    cull_mode: vk::CullModeFlags::BACK,
                    front_face: vk::FrontFace::CLOCKWISE,
                },
                descriptor_sets_infos: vec![DescriptorSetCreateInfo {
                    descriptor_infos: vec![
                        DescriptorBindingCreateInfo {
                            descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                            shader_stage_flags: vk::ShaderStageFlags::FRAGMENT,
                            buffer_info: None,
                            image_info: Some(
                                vk::DescriptorImageInfoBuilder::new()
                                    .image_view(scene_buffers.textures_image.view)
                                    .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL),
                            ),
                            sampler_index: Some(0),
                        },
                        DescriptorBindingCreateInfo {
                            descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                            shader_stage_flags: vk::ShaderStageFlags::FRAGMENT,
                            buffer_info: None,
                            image_info: Some(
                                vk::DescriptorImageInfoBuilder::new()
                                    .image_view(scene_buffers.normal_maps_image.view)
                                    .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL),
                            ),
                            sampler_index: Some(0),
                        },
                        DescriptorBindingCreateInfo {
                            descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                            shader_stage_flags: vk::ShaderStageFlags::FRAGMENT,
                            buffer_info: Some(
                                vk::DescriptorBufferInfoBuilder::new()
                                    .buffer(camera_buffer.buffer)
                                    .range(vk::WHOLE_SIZE),
                            ),
                            image_info: None,
                            sampler_index: None,
                        },
                        DescriptorBindingCreateInfo {
                            descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                            shader_stage_flags: vk::ShaderStageFlags::FRAGMENT,
                            buffer_info: Some(
                                vk::DescriptorBufferInfoBuilder::new()
                                    .buffer(scene_buffers.bih_buffer.buffer)
                                    .range(vk::WHOLE_SIZE),
                            ),
                            image_info: None,
                            sampler_index: None,
                        },
                        DescriptorBindingCreateInfo {
                            descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                            shader_stage_flags: vk::ShaderStageFlags::FRAGMENT,
                            buffer_info: Some(
                                vk::DescriptorBufferInfoBuilder::new()
                                    .buffer(scene_buffers.materials_buffer.buffer)
                                    .range(vk::WHOLE_SIZE),
                            ),
                            image_info: None,
                            sampler_index: None,
                        },
                        DescriptorBindingCreateInfo {
                            descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                            shader_stage_flags: vk::ShaderStageFlags::FRAGMENT,
                            buffer_info: Some(
                                vk::DescriptorBufferInfoBuilder::new()
                                    .buffer(scene_buffers.triangles_buffer.buffer)
                                    .range(vk::WHOLE_SIZE),
                            ),
                            image_info: None,
                            sampler_index: None,
                        },
                    ],
                }],
                sampler_infos: vec![SamplerCreateInfo {
                    address_mode: vk::SamplerAddressMode::REPEAT,
                    filter: vk::Filter::LINEAR,
                }],
            },
        );

        Self {
            vertex_buffer,
            push_constants: PushConstants {
                n_triangles: std140::uint(scene_buffers.n_triangles),
                use_bih: std140::boolean::True,
            },
            graphics_pipeline,
        }
    }

    pub fn destroy(&self, device: &DeviceLoader) {
        self.vertex_buffer.destroy();
        self.graphics_pipeline.destroy(device);
    }

    pub fn render(&self, ctx: RenderContext) {
        unsafe {
            ctx.device.cmd_bind_pipeline(
                ctx.command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.graphics_pipeline.pipeline,
            );
            ctx.device.cmd_bind_descriptor_sets(
                ctx.command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.graphics_pipeline.layout,
                0,
                &self.graphics_pipeline.descriptor_sets,
                &[],
            );
            ctx.device.cmd_push_constants(
                ctx.command_buffer,
                self.graphics_pipeline.layout,
                vk::ShaderStageFlags::FRAGMENT,
                0,
                std::mem::size_of::<PushConstants>() as u32,
                &self.push_constants as *const PushConstants as *const c_void,
            );
            ctx.device.cmd_bind_vertex_buffers(
                ctx.command_buffer,
                0,
                &[self.vertex_buffer.buffer],
                &[0],
            );
            ctx.device.cmd_draw(ctx.command_buffer, 4, 1, 0, 0);
        }
    }
}
