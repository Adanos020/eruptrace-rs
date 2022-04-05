use crate::shaders::{SURFACE_FRAGMENT_SHADER, SURFACE_VERTEX_SHADER};
use erupt::{vk, DeviceLoader, ExtendableFrom, SmallVec};
use eruptrace_vk::contexts::RenderContext;
use eruptrace_vk::{
    shader::make_shader_module, AllocatedBuffer, AllocatedImage, PipelineContext, VulkanContext,
};
use nalgebra_glm as glm;
use std::{
    ffi::CString,
    sync::{Arc, RwLock},
};
use itertools::Itertools;
use vk_mem_erupt as vma;

#[derive(Copy, Clone, Debug)]
struct Vertex {
    position: glm::Vec2,
    texcoords: glm::Vec2,
}

#[derive(Clone)]
pub struct RenderSurface {
    allocator: Arc<RwLock<vma::Allocator>>,

    frames_in_flight: u32,
    frames_since_resize: u32,
    images_to_delete: Vec<AllocatedImage>,

    vertex_buffer: AllocatedBuffer<Vertex>,
    pub render_image: AllocatedImage,

    graphics_pipeline_layout: vk::PipelineLayout,
    graphics_pipeline: vk::Pipeline,

    sampler: vk::Sampler,
    descriptor_set_layouts: Vec<vk::DescriptorSetLayout>,
    descriptor_pool: vk::DescriptorPool,
    descriptor_sets: SmallVec<vk::DescriptorSet>,
}

impl RenderSurface {
    pub fn new(
        allocator: Arc<RwLock<vma::Allocator>>,
        vk_ctx: VulkanContext,
        pipeline_ctx: PipelineContext,
        frames_in_flight: u32,
    ) -> vma::Result<Self> {
        let vertex_buffer = {
            let vertices = [
                Vertex {
                    position: glm::vec2(-1.0, -1.0),
                    texcoords: glm::vec2(0.0, 0.0),
                },
                Vertex {
                    position: glm::vec2(1.0, -1.0),
                    texcoords: glm::vec2(1.0, 0.0),
                },
                Vertex {
                    position: glm::vec2(-1.0, 1.0),
                    texcoords: glm::vec2(0.0, 1.0),
                },
                Vertex {
                    position: glm::vec2(1.0, 1.0),
                    texcoords: glm::vec2(1.0, 1.0),
                },
            ];

            let buffer_info = vk::BufferCreateInfoBuilder::new()
                .usage(vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);
            AllocatedBuffer::with_data(
                allocator.clone(),
                &buffer_info,
                vma::MemoryUsage::CpuToGpu,
                &vertices,
            )
        };

        let render_image = AllocatedImage::texture(
            vk_ctx.clone(),
            allocator.clone(),
            vk::Extent3D {
                width: 1,
                height: 1,
                depth: 1,
            },
            vk::ImageViewType::_2D,
            1,
            1,
            &[0u8, 0u8, 0u8, 0u8],
        );

        let descriptor_set_layouts = {
            let bindings = vec![vk::DescriptorSetLayoutBindingBuilder::new()
                .binding(0)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT)];
            let create_info = vk::DescriptorSetLayoutCreateInfoBuilder::new().bindings(&bindings);
            let layout = unsafe {
                vk_ctx
                    .device
                    .create_descriptor_set_layout(&create_info, None)
                    .expect("Cannot create descriptor set layout")
            };
            vec![layout]
        };

        let descriptor_pool = {
            let sizes = vec![vk::DescriptorPoolSizeBuilder::new()
                ._type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(10)];
            let create_info = vk::DescriptorPoolCreateInfoBuilder::new()
                .max_sets(10)
                .pool_sizes(&sizes);
            unsafe {
                vk_ctx
                    .device
                    .create_descriptor_pool(&create_info, None)
                    .expect("Cannot create descriptor pool")
            }
        };

        let descriptor_sets = {
            let allocate_info = vk::DescriptorSetAllocateInfoBuilder::new()
                .descriptor_pool(descriptor_pool)
                .set_layouts(&descriptor_set_layouts);
            unsafe {
                vk_ctx
                    .device
                    .allocate_descriptor_sets(&allocate_info)
                    .expect("Cannot allocate descriptor sets")
            }
        };

        let sampler = {
            let create_info = vk::SamplerCreateInfoBuilder::new()
                .address_mode_u(vk::SamplerAddressMode::REPEAT)
                .address_mode_v(vk::SamplerAddressMode::REPEAT)
                .address_mode_w(vk::SamplerAddressMode::REPEAT)
                .min_filter(vk::Filter::NEAREST)
                .mag_filter(vk::Filter::NEAREST);
            unsafe {
                vk_ctx
                    .device
                    .create_sampler(&create_info, None)
                    .expect("Cannot create sampler")
            }
        };

        let image_infos = vec![vk::DescriptorImageInfoBuilder::new()
            .image_view(render_image.view)
            .sampler(sampler)
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];

        let descriptor_writes = vec![vk::WriteDescriptorSetBuilder::new()
            .dst_binding(0)
            .dst_set(descriptor_sets[0])
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(&image_infos)];

        unsafe {
            vk_ctx
                .device
                .update_descriptor_sets(&descriptor_writes, &[]);
        }

        let vertex_shader = make_shader_module(&vk_ctx.device, SURFACE_VERTEX_SHADER);
        let fragment_shader = make_shader_module(&vk_ctx.device, SURFACE_FRAGMENT_SHADER);

        let entry_point = CString::new("main").unwrap();
        let shader_stages = vec![
            vk::PipelineShaderStageCreateInfoBuilder::new()
                .stage(vk::ShaderStageFlagBits::VERTEX)
                .module(vertex_shader)
                .name(&entry_point),
            vk::PipelineShaderStageCreateInfoBuilder::new()
                .stage(vk::ShaderStageFlagBits::FRAGMENT)
                .module(fragment_shader)
                .name(&entry_point),
        ];

        let mut pipeline_rendering_info = vk::PipelineRenderingCreateInfoBuilder::new()
            .color_attachment_formats(std::slice::from_ref(&pipeline_ctx.surface_format.format));

        let graphics_pipeline_layout = {
            let create_info =
                vk::PipelineLayoutCreateInfoBuilder::new().set_layouts(&descriptor_set_layouts);
            unsafe {
                vk_ctx
                    .device
                    .create_pipeline_layout(&create_info, None)
                    .expect("Cannot create graphics pipeline layout")
            }
        };

        let input_assembly = vk::PipelineInputAssemblyStateCreateInfoBuilder::new()
            .topology(vk::PrimitiveTopology::TRIANGLE_STRIP)
            .primitive_restart_enable(false);

        let dynamic_pipeline_state = vk::PipelineDynamicStateCreateInfoBuilder::new()
            .dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]);

        let viewport_state = vk::PipelineViewportStateCreateInfoBuilder::new()
            .viewport_count(1)
            .scissor_count(1);

        let rasterisation_state = vk::PipelineRasterizationStateCreateInfoBuilder::new()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::CLOCKWISE)
            .depth_clamp_enable(false);

        let multisample_state = vk::PipelineMultisampleStateCreateInfoBuilder::new()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlagBits::_1);

        let colour_blend_attachments = vec![vk::PipelineColorBlendAttachmentStateBuilder::new()
            .color_write_mask(
                vk::ColorComponentFlags::R
                    | vk::ColorComponentFlags::G
                    | vk::ColorComponentFlags::B
                    | vk::ColorComponentFlags::A,
            )
            .blend_enable(false)];
        let colour_blending_info = vk::PipelineColorBlendStateCreateInfoBuilder::new()
            .logic_op_enable(false)
            .attachments(&colour_blend_attachments);

        let binding_descriptions = [vk::VertexInputBindingDescriptionBuilder::new()
            .binding(0)
            .input_rate(vk::VertexInputRate::VERTEX)
            .stride(std::mem::size_of::<Vertex>() as u32)];
        let attribute_descriptions = [
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
        ];
        let vertex_input = vk::PipelineVertexInputStateCreateInfoBuilder::new()
            .vertex_binding_descriptions(&binding_descriptions)
            .vertex_attribute_descriptions(&attribute_descriptions);

        let pipeline_infos = vec![vk::GraphicsPipelineCreateInfoBuilder::new()
            .vertex_input_state(&vertex_input)
            .color_blend_state(&colour_blending_info)
            .multisample_state(&multisample_state)
            .stages(&shader_stages)
            .layout(graphics_pipeline_layout)
            .rasterization_state(&rasterisation_state)
            .dynamic_state(&dynamic_pipeline_state)
            .viewport_state(&viewport_state)
            .input_assembly_state(&input_assembly)
            .extend_from(&mut pipeline_rendering_info)];

        let graphics_pipeline = unsafe {
            vk_ctx
                .device
                .create_graphics_pipelines(vk::PipelineCache::null(), &pipeline_infos, None)
                .expect("Cannot create graphics pipeline")[0]
        };

        unsafe {
            vk_ctx.device.destroy_shader_module(vertex_shader, None);
            vk_ctx.device.destroy_shader_module(fragment_shader, None);
        }

        Ok(Self {
            allocator,
            frames_in_flight,
            frames_since_resize: 0,
            images_to_delete: vec![],
            vertex_buffer,
            render_image,
            graphics_pipeline_layout,
            graphics_pipeline,
            sampler,
            descriptor_set_layouts,
            descriptor_pool,
            descriptor_sets,
        })
    }

    pub fn destroy(&self, device: &DeviceLoader) {
        self.vertex_buffer.destroy();
        self.render_image.destroy(device);
        for image in self.images_to_delete.iter() {
            image.destroy(device);
        }
        unsafe {
            for &layout in self.descriptor_set_layouts.iter() {
                device.destroy_descriptor_set_layout(layout, None);
            }
            device.destroy_descriptor_pool(self.descriptor_pool, None);
            device.destroy_sampler(self.sampler, None);
            device.destroy_pipeline(self.graphics_pipeline, None);
            device.destroy_pipeline_layout(self.graphics_pipeline_layout, None);
        }
    }

    pub fn update_image_size(&mut self, vk_ctx: VulkanContext, extent: vk::Extent2D) {
        let texture_data = (0 .. 4 * extent.width * extent.height).map(|_| 0u8).collect_vec();
        let new_image = AllocatedImage::texture(
            vk_ctx.clone(),
            self.allocator.clone(),
            vk::Extent3D {
                width: extent.width,
                height: extent.height,
                depth: 1,
            },
            vk::ImageViewType::_2D,
            1,
            1,
            &texture_data,
        );
        let to_delete = std::mem::replace(&mut self.render_image, new_image);
        self.images_to_delete.push(to_delete);

        let image_infos = vec![vk::DescriptorImageInfoBuilder::new()
            .image_view(self.render_image.view)
            .sampler(self.sampler)
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];

        let descriptor_writes = vec![vk::WriteDescriptorSetBuilder::new()
            .dst_binding(0)
            .dst_set(self.descriptor_sets[0])
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(&image_infos)];

        unsafe {
            vk_ctx
                .device
                .update_descriptor_sets(&descriptor_writes, &[]);
        }
    }

    pub fn render(&mut self, ctx: RenderContext) {
        if self.frames_since_resize <= self.frames_in_flight {
            self.frames_since_resize += 1;
            if self.frames_since_resize == self.frames_in_flight {
                for image in self.images_to_delete.drain(..) {
                    image.destroy(&ctx.device);
                }
            }
        }

        unsafe {
            ctx.device.cmd_bind_pipeline(
                ctx.command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.graphics_pipeline,
            );
            ctx.device.cmd_bind_descriptor_sets(
                ctx.command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.graphics_pipeline_layout,
                0,
                &self.descriptor_sets,
                &[],
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
