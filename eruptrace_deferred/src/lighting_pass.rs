use std::ffi::c_void;

use erupt::{vk, DeviceLoader};
use eruptrace_scene::{CameraUniform, SceneBuffers};
use eruptrace_vk::{
    command,
    pipeline::{
        ColorAttachmentInfo,
        DescriptorBindingCreateInfo,
        DescriptorSetCreateInfo,
        GraphicsPipelineCreateInfo,
        Pipeline,
        RasterisationStateInfo,
        SamplerCreateInfo,
    },
    push_constants::RtPushConstants,
    AllocatedBuffer,
    AllocatedImage,
    VulkanContext,
};
use nalgebra_glm as glm;
use vk_mem_erupt as vma;

use crate::{
    gbuffers::GBuffers,
    shaders::{LIGHTING_FRAGMENT_SHADER, LIGHTING_VERTEX_SHADER},
};

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct Vertex {
    pub position: glm::Vec2,
}

#[derive(Clone)]
pub struct LightingPass {
    vertex_buffer:     AllocatedBuffer<Vertex>,
    output_extent:     vk::Extent2D,
    graphics_pipeline: Pipeline,
}

impl LightingPass {
    pub fn new(
        vk_ctx: VulkanContext,
        output_extent: vk::Extent2D,
        gbuffers: &GBuffers,
        camera_buffer: &AllocatedBuffer<CameraUniform>,
        scene_buffers: &SceneBuffers,
    ) -> Self {
        let vertex_buffer = {
            let vertices = [
                Vertex { position: glm::vec2(-1.0, -1.0) },
                Vertex { position: glm::vec2(1.0, -1.0) },
                Vertex { position: glm::vec2(-1.0, 1.0) },
                Vertex { position: glm::vec2(1.0, 1.0) },
            ];

            let buffer_info = vk::BufferCreateInfoBuilder::new()
                .usage(vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);
            AllocatedBuffer::with_data(vk_ctx.allocator.clone(), &buffer_info, vma::MemoryUsage::CpuToGpu, &vertices)
        };

        let graphics_pipeline = Pipeline::graphics(vk_ctx, GraphicsPipelineCreateInfo {
            vertex_shader:           LIGHTING_VERTEX_SHADER,
            fragment_shader:         LIGHTING_FRAGMENT_SHADER,
            color_attachment_infos:  vec![ColorAttachmentInfo {
                format:           vk::Format::R8G8B8A8_UNORM,
                color_write_mask: vk::ColorComponentFlags::all(),
            }],
            push_constant_ranges:    vec![vk::PushConstantRangeBuilder::new()
                .offset(0)
                .size(std::mem::size_of::<RtPushConstants>() as u32)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT)],
            input_assembly:          vk::PipelineInputAssemblyStateCreateInfoBuilder::new()
                .topology(vk::PrimitiveTopology::TRIANGLE_STRIP)
                .primitive_restart_enable(false),
            vertex_input_bindings:   vec![vk::VertexInputBindingDescriptionBuilder::new()
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
            rasterisation_state:     RasterisationStateInfo {
                cull_mode:  vk::CullModeFlags::BACK,
                front_face: vk::FrontFace::CLOCKWISE,
            },
            descriptor_sets_infos:   vec![
                DescriptorSetCreateInfo {
                    descriptor_infos: vec![
                        DescriptorBindingCreateInfo::image(
                            vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                            vk::ShaderStageFlags::FRAGMENT,
                            vk::DescriptorImageInfoBuilder::new()
                                .image_view(gbuffers.out_positions.view)
                                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL),
                            0,
                        ),
                        DescriptorBindingCreateInfo::image(
                            vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                            vk::ShaderStageFlags::FRAGMENT,
                            vk::DescriptorImageInfoBuilder::new()
                                .image_view(gbuffers.out_normals.view)
                                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL),
                            0,
                        ),
                        DescriptorBindingCreateInfo::image(
                            vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                            vk::ShaderStageFlags::FRAGMENT,
                            vk::DescriptorImageInfoBuilder::new()
                                .image_view(gbuffers.out_materials.view)
                                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL),
                            0,
                        ),
                    ],
                },
                DescriptorSetCreateInfo {
                    descriptor_infos: vec![
                        DescriptorBindingCreateInfo::image(
                            vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                            vk::ShaderStageFlags::FRAGMENT,
                            vk::DescriptorImageInfoBuilder::new()
                                .image_view(scene_buffers.textures_image.view)
                                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL),
                            1,
                        ),
                        DescriptorBindingCreateInfo::image(
                            vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                            vk::ShaderStageFlags::FRAGMENT,
                            vk::DescriptorImageInfoBuilder::new()
                                .image_view(scene_buffers.normal_maps_image.view)
                                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL),
                            1,
                        ),
                        DescriptorBindingCreateInfo::buffer(
                            vk::DescriptorType::UNIFORM_BUFFER,
                            vk::ShaderStageFlags::FRAGMENT,
                            vk::DescriptorBufferInfoBuilder::new().buffer(camera_buffer.buffer).range(vk::WHOLE_SIZE),
                        ),
                        DescriptorBindingCreateInfo::buffer(
                            vk::DescriptorType::STORAGE_BUFFER,
                            vk::ShaderStageFlags::FRAGMENT,
                            vk::DescriptorBufferInfoBuilder::new()
                                .buffer(scene_buffers.bih_buffer.buffer)
                                .range(vk::WHOLE_SIZE),
                        ),
                        DescriptorBindingCreateInfo::buffer(
                            vk::DescriptorType::STORAGE_BUFFER,
                            vk::ShaderStageFlags::FRAGMENT,
                            vk::DescriptorBufferInfoBuilder::new()
                                .buffer(scene_buffers.materials_buffer.buffer)
                                .range(vk::WHOLE_SIZE),
                        ),
                        DescriptorBindingCreateInfo::buffer(
                            vk::DescriptorType::STORAGE_BUFFER,
                            vk::ShaderStageFlags::FRAGMENT,
                            vk::DescriptorBufferInfoBuilder::new()
                                .buffer(scene_buffers.triangles_buffer.buffer)
                                .range(vk::WHOLE_SIZE),
                        ),
                    ],
                },
            ],
            sampler_infos:           vec![
                SamplerCreateInfo {
                    address_mode: vk::SamplerAddressMode::CLAMP_TO_EDGE,
                    filter:       vk::Filter::NEAREST,
                },
                SamplerCreateInfo { address_mode: vk::SamplerAddressMode::REPEAT, filter: vk::Filter::LINEAR },
            ],
            enable_depth_testing:    false,
        });

        Self { vertex_buffer, output_extent, graphics_pipeline }
    }

    pub fn destroy(&self, device: &DeviceLoader) {
        self.vertex_buffer.destroy();
        self.graphics_pipeline.destroy(device);
    }

    pub fn update_output(&mut self, device: &DeviceLoader, extent: vk::Extent2D, gbuffers: &GBuffers) {
        self.output_extent = extent;

        let write = vk::WriteDescriptorSetBuilder::new()
            .dst_set(self.graphics_pipeline.descriptor_sets[0])
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER);
        let info = vk::DescriptorImageInfoBuilder::new()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .sampler(self.graphics_pipeline.samplers[0]);
        let image_infos = vec![
            info.image_view(gbuffers.out_positions.view),
            info.image_view(gbuffers.out_normals.view),
            info.image_view(gbuffers.out_materials.view),
        ];
        let descriptor_writes = vec![write.dst_binding(0).image_info(&image_infos)];

        unsafe {
            device.update_descriptor_sets(&descriptor_writes, &[]);
        }
    }

    pub fn render(&self, vk_ctx: VulkanContext, push_constants: &RtPushConstants, target: &AllocatedImage) {
        command::immediate_submit(vk_ctx, |device, command_buffer| unsafe {
            command::set_scissor_and_viewport(device, command_buffer, self.output_extent);

            device.cmd_pipeline_barrier2(
                command_buffer,
                &vk::DependencyInfoBuilder::new().image_memory_barriers(&[vk::ImageMemoryBarrier2Builder::new()
                    .src_stage_mask(vk::PipelineStageFlags2::FRAGMENT_SHADER)
                    .dst_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
                    .src_access_mask(vk::AccessFlags2::NONE)
                    .dst_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
                    .old_layout(vk::ImageLayout::UNDEFINED)
                    .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .image(target.image)
                    .subresource_range(target.subresource_range)]),
            );

            device.cmd_begin_rendering(
                command_buffer,
                &vk::RenderingInfoBuilder::new()
                    .color_attachments(&[vk::RenderingAttachmentInfoBuilder::new()
                        .image_view(target.view)
                        .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                        .clear_value(vk::ClearValue { color: vk::ClearColorValue { float32: [0.0, 0.0, 0.0, 0.0] } })
                        .load_op(vk::AttachmentLoadOp::CLEAR)
                        .store_op(vk::AttachmentStoreOp::STORE)
                        .resolve_image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)])
                    .layer_count(1)
                    .render_area(vk::Rect2D { offset: Default::default(), extent: self.output_extent }),
            );
            device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, self.graphics_pipeline.pipeline);
            device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.graphics_pipeline.layout,
                0,
                &self.graphics_pipeline.descriptor_sets,
                &[],
            );
            device.cmd_push_constants(
                command_buffer,
                self.graphics_pipeline.layout,
                vk::ShaderStageFlags::FRAGMENT,
                0,
                std::mem::size_of::<RtPushConstants>() as u32,
                push_constants as *const RtPushConstants as *const c_void,
            );
            device.cmd_bind_vertex_buffers(command_buffer, 0, &[self.vertex_buffer.buffer], &[0]);
            device.cmd_draw(command_buffer, 4, 1, 0, 0);
            device.cmd_end_rendering(command_buffer);

            device.cmd_pipeline_barrier2(
                command_buffer,
                &vk::DependencyInfoBuilder::new().image_memory_barriers(&[vk::ImageMemoryBarrier2Builder::new()
                    .src_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
                    .dst_stage_mask(vk::PipelineStageFlags2::FRAGMENT_SHADER)
                    .src_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE_KHR)
                    .dst_access_mask(vk::AccessFlags2::SHADER_SAMPLED_READ)
                    .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                    .image(target.image)
                    .subresource_range(target.subresource_range)]),
            );
        });
    }
}
