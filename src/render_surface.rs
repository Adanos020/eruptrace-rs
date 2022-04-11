use erupt::{vk, DeviceLoader};
use eruptrace_vk::{
    contexts::RenderContext,
    pipeline::{
        ColorAttachmentInfo,
        DescriptorBindingCreateInfo,
        DescriptorSetCreateInfo,
        GraphicsPipelineCreateInfo,
        Pipeline,
        RasterisationStateInfo,
        SamplerCreateInfo,
    },
    AllocatedBuffer,
    AllocatedImage,
    PipelineContext,
    VulkanContext,
};
use itertools::Itertools;
use nalgebra_glm as glm;
use vk_mem_erupt as vma;

use crate::shaders::{SURFACE_FRAGMENT_SHADER, SURFACE_VERTEX_SHADER};

#[allow(unused)]
#[derive(Copy, Clone, Debug)]
struct Vertex {
    position:  glm::Vec2,
    texcoords: glm::Vec2,
}

#[derive(Clone)]
pub struct RenderSurface {
    frames_in_flight:    u32,
    frames_since_resize: u32,
    images_to_delete:    Vec<AllocatedImage>,

    vertex_buffer:    AllocatedBuffer<Vertex>,
    pub render_image: AllocatedImage,

    graphics_pipeline: Pipeline,
}

impl RenderSurface {
    pub fn new(vk_ctx: VulkanContext, pipeline_ctx: PipelineContext, frames_in_flight: u32) -> vma::Result<Self> {
        let vertex_buffer = {
            let vertices = [
                Vertex { position: glm::vec2(-1.0, -1.0), texcoords: glm::vec2(0.0, 0.0) },
                Vertex { position: glm::vec2(1.0, -1.0), texcoords: glm::vec2(1.0, 0.0) },
                Vertex { position: glm::vec2(-1.0, 1.0), texcoords: glm::vec2(0.0, 1.0) },
                Vertex { position: glm::vec2(1.0, 1.0), texcoords: glm::vec2(1.0, 1.0) },
            ];

            let buffer_info = vk::BufferCreateInfoBuilder::new()
                .usage(vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);
            AllocatedBuffer::with_data(vk_ctx.allocator.clone(), &buffer_info, vma::MemoryUsage::CpuToGpu, &vertices)
        };

        let render_image = {
            let image_info = vk::ImageCreateInfoBuilder::new()
                .usage(
                    vk::ImageUsageFlags::COLOR_ATTACHMENT
                        | vk::ImageUsageFlags::TRANSFER_DST
                        | vk::ImageUsageFlags::SAMPLED,
                )
                .format(vk::Format::R8G8B8A8_UNORM)
                .extent(vk::Extent3D { width: 1, height: 1, depth: 1 })
                .mip_levels(1)
                .array_layers(1)
                .samples(vk::SampleCountFlagBits::_1)
                .image_type(vk::ImageType::_2D);

            let range = vk::ImageSubresourceRangeBuilder::new()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_mip_level(0)
                .base_array_layer(0)
                .level_count(1)
                .layer_count(1)
                .build();

            AllocatedImage::with_data(vk_ctx.clone(), image_info, vk::ImageViewType::_2D, range, &[0u8, 0u8, 0u8, 0u8])
        };

        let graphics_pipeline = Pipeline::graphics(vk_ctx, GraphicsPipelineCreateInfo {
            vertex_shader:           SURFACE_VERTEX_SHADER,
            fragment_shader:         SURFACE_FRAGMENT_SHADER,
            color_attachment_infos:  vec![ColorAttachmentInfo {
                format:           pipeline_ctx.surface_format.format,
                color_write_mask: vk::ColorComponentFlags::all(),
            }],
            push_constant_ranges:    vec![],
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
                // texCoord
                vk::VertexInputAttributeDescriptionBuilder::new()
                    .binding(0)
                    .location(1)
                    .format(vk::Format::R32G32_SFLOAT)
                    .offset(std::mem::size_of::<glm::Vec2>() as u32),
            ],
            rasterisation_state:     RasterisationStateInfo {
                cull_mode:  vk::CullModeFlags::BACK,
                front_face: vk::FrontFace::CLOCKWISE,
            },
            descriptor_sets_infos:   vec![DescriptorSetCreateInfo {
                descriptor_infos: vec![DescriptorBindingCreateInfo::image(
                    vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    vk::ShaderStageFlags::FRAGMENT,
                    vk::DescriptorImageInfoBuilder::new()
                        .image_view(render_image.view)
                        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL),
                    0,
                )],
            }],
            sampler_infos:           vec![SamplerCreateInfo {
                address_mode: vk::SamplerAddressMode::REPEAT,
                filter:       vk::Filter::NEAREST,
            }],
            enable_depth_testing:    false,
        });

        Ok(Self {
            frames_in_flight,
            frames_since_resize: 0,
            images_to_delete: vec![],
            vertex_buffer,
            render_image,
            graphics_pipeline,
        })
    }

    pub fn destroy(&self, device: &DeviceLoader) {
        self.vertex_buffer.destroy();
        self.render_image.destroy(device);
        for image in self.images_to_delete.iter() {
            image.destroy(device);
        }
        self.graphics_pipeline.destroy(device);
    }

    pub fn update_image_size(&mut self, vk_ctx: VulkanContext, extent: vk::Extent2D) {
        let texture_data = (0..4 * extent.width * extent.height).map(|_| 0u8).collect_vec();
        let new_image = {
            let image_info = vk::ImageCreateInfoBuilder::new()
                .usage(
                    vk::ImageUsageFlags::COLOR_ATTACHMENT
                        | vk::ImageUsageFlags::TRANSFER_DST
                        | vk::ImageUsageFlags::SAMPLED,
                )
                .format(vk::Format::R8G8B8A8_UNORM)
                .extent(vk::Extent3D { width: extent.width, height: extent.height, depth: 1 })
                .mip_levels(1)
                .array_layers(1)
                .samples(vk::SampleCountFlagBits::_1)
                .image_type(vk::ImageType::_2D);

            let range = vk::ImageSubresourceRangeBuilder::new()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_mip_level(0)
                .base_array_layer(0)
                .level_count(1)
                .layer_count(1)
                .build();

            AllocatedImage::with_data(vk_ctx.clone(), image_info, vk::ImageViewType::_2D, range, &texture_data)
        };
        let to_delete = std::mem::replace(&mut self.render_image, new_image);
        self.images_to_delete.push(to_delete);
        self.frames_since_resize = 0;

        let image_infos = vec![vk::DescriptorImageInfoBuilder::new()
            .image_view(self.render_image.view)
            .sampler(self.graphics_pipeline.samplers[0])
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];

        let descriptor_writes = vec![vk::WriteDescriptorSetBuilder::new()
            .dst_binding(0)
            .dst_set(self.graphics_pipeline.descriptor_sets[0])
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(&image_infos)];

        unsafe {
            vk_ctx.device.update_descriptor_sets(&descriptor_writes, &[]);
        }
    }

    pub fn render(&mut self, ctx: RenderContext) {
        if self.frames_since_resize < self.frames_in_flight {
            self.frames_since_resize += 1;
            if self.frames_since_resize == self.frames_in_flight {
                for image in self.images_to_delete.drain(..) {
                    image.destroy(ctx.device);
                }
            }
        }

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
            ctx.device.cmd_bind_vertex_buffers(ctx.command_buffer, 0, &[self.vertex_buffer.buffer], &[0]);
            ctx.device.cmd_draw(ctx.command_buffer, 4, 1, 0, 0);
        }
    }
}
