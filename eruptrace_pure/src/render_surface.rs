use crate::scene::SceneBuffers;
use crate::shaders::*;
use crate::CameraUniform;
use erupt::{vk, DeviceLoader, ExtendableFrom, SmallVec};
use eruptrace_vk::contexts::{PipelineContext, RenderContext};
use eruptrace_vk::{shader::make_shader_module, AllocatedBuffer, VulkanContext};
use nalgebra_glm as glm;
use std::{
    ffi::CString,
    sync::{Arc, RwLock},
};
use vk_mem_erupt as vma;

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct Vertex {
    pub position: glm::TVec2<f32>,
}

#[derive(Clone)]
pub struct RenderSurface {
    vertex_buffer: AllocatedBuffer<Vertex>,

    vertex_shader: vk::ShaderModule,
    fragment_shader: vk::ShaderModule,
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
        camera_buffer: &AllocatedBuffer<CameraUniform>,
        scene_buffers: &SceneBuffers,
    ) -> Self {
        let descriptor_set_layouts: Vec<_> = {
            let bindings = vec![
                vk::DescriptorSetLayoutBindingBuilder::new()
                    .binding(0)
                    .descriptor_count(1)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .stage_flags(vk::ShaderStageFlags::FRAGMENT),
                vk::DescriptorSetLayoutBindingBuilder::new()
                    .binding(1)
                    .descriptor_count(1)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .stage_flags(vk::ShaderStageFlags::FRAGMENT),
                vk::DescriptorSetLayoutBindingBuilder::new()
                    .binding(2)
                    .descriptor_count(1)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .stage_flags(vk::ShaderStageFlags::FRAGMENT),
                // vk::DescriptorSetLayoutBindingBuilder::new()
                //     .binding(3)
                //     .descriptor_count(1)
                //     .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                //     .stage_flags(vk::ShaderStageFlags::FRAGMENT),
                vk::DescriptorSetLayoutBindingBuilder::new()
                    .binding(4)
                    .descriptor_count(1)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .stage_flags(vk::ShaderStageFlags::FRAGMENT),
                vk::DescriptorSetLayoutBindingBuilder::new()
                    .binding(5)
                    .descriptor_count(1)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .stage_flags(vk::ShaderStageFlags::FRAGMENT),
                vk::DescriptorSetLayoutBindingBuilder::new()
                    .binding(6)
                    .descriptor_count(1)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .stage_flags(vk::ShaderStageFlags::FRAGMENT),
            ];
            vec![{
                let create_info =
                    vk::DescriptorSetLayoutCreateInfoBuilder::new().bindings(&bindings);
                unsafe {
                    vk_ctx
                        .device
                        .create_descriptor_set_layout(&create_info, None)
                        .expect("Cannot create descriptor set layout")
                }
            }]
        };

        let descriptor_pool = {
            let sizes: Vec<_> = vec![
                vk::DescriptorPoolSizeBuilder::new()
                    ._type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .descriptor_count(10),
                vk::DescriptorPoolSizeBuilder::new()
                    ._type(vk::DescriptorType::UNIFORM_BUFFER)
                    .descriptor_count(10),
                vk::DescriptorPoolSizeBuilder::new()
                    ._type(vk::DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(10),
            ];
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
                .min_filter(vk::Filter::LINEAR)
                .mag_filter(vk::Filter::LINEAR);
            unsafe {
                vk_ctx
                    .device
                    .create_sampler(&create_info, None)
                    .expect("Cannot create sampler")
            }
        };

        let image_infos: Vec<_> = {
            [
                scene_buffers.textures_image.view,
                scene_buffers.normal_maps_image.view,
            ]
            .into_iter()
            .map(|view| {
                vk::DescriptorImageInfoBuilder::new()
                    .image_view(view)
                    .sampler(sampler)
                    .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            })
            .collect()
        };

        let uniform_buffer_infos = vec![vk::DescriptorBufferInfoBuilder::new()
            .buffer(camera_buffer.buffer)
            .range(vk::WHOLE_SIZE)];

        let storage_buffer_infos = vec![
            vk::DescriptorBufferInfoBuilder::new()
                .buffer(scene_buffers.materials_buffer.buffer)
                .range(vk::WHOLE_SIZE),
            // vk::DescriptorBufferInfoBuilder::new()
            //     .buffer(scene_buffers.bih_buffer.buffer)
            //     .range(vk::WHOLE_SIZE),
            vk::DescriptorBufferInfoBuilder::new()
                .buffer(scene_buffers.mesh_metas_buffer.buffer)
                .range(vk::WHOLE_SIZE),
            vk::DescriptorBufferInfoBuilder::new()
                .buffer(scene_buffers.mesh_data_buffer.buffer)
                .range(vk::WHOLE_SIZE),
        ];

        let descriptor_set_writes = vec![
            vk::WriteDescriptorSetBuilder::new()
                .dst_binding(0)
                .dst_set(descriptor_sets[0])
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(&image_infos),
            vk::WriteDescriptorSetBuilder::new()
                .dst_binding(2)
                .dst_set(descriptor_sets[0])
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(&uniform_buffer_infos),
            vk::WriteDescriptorSetBuilder::new()
                .dst_binding(4) // TODO change back to 3
                .dst_set(descriptor_sets[0])
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(&storage_buffer_infos),
        ];

        unsafe {
            vk_ctx
                .device
                .update_descriptor_sets(&descriptor_set_writes, &[]);
        }

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
            let allocation_info = vma::AllocationCreateInfo {
                usage: vma::MemoryUsage::CpuToGpu,
                flags: vma::AllocationCreateFlags::DEDICATED_MEMORY
                    | vma::AllocationCreateFlags::MAPPED,
                ..Default::default()
            };

            AllocatedBuffer::with_data(allocator, &buffer_info, allocation_info, &vertices)
                .expect("Cannot create vertex buffer")
        };

        let vertex_shader = make_shader_module(&vk_ctx.device, VERTEX_SHADER);
        let fragment_shader = make_shader_module(&vk_ctx.device, FRAGMENT_SHADER);

        let entry_point = CString::new("main").unwrap();
        let shader_stages = {
            vec![
                vk::PipelineShaderStageCreateInfoBuilder::new()
                    .stage(vk::ShaderStageFlagBits::VERTEX)
                    .module(vertex_shader)
                    .name(&entry_point),
                vk::PipelineShaderStageCreateInfoBuilder::new()
                    .stage(vk::ShaderStageFlagBits::FRAGMENT)
                    .module(fragment_shader)
                    .name(&entry_point),
            ]
        };

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

        Self {
            vertex_buffer,
            vertex_shader,
            fragment_shader,
            graphics_pipeline_layout,
            graphics_pipeline,
            sampler,
            descriptor_set_layouts,
            descriptor_pool,
            descriptor_sets,
        }
    }

    pub fn destroy(&self, device: &DeviceLoader) {
        self.vertex_buffer.destroy();
        unsafe {
            for &layout in self.descriptor_set_layouts.iter() {
                device.destroy_descriptor_set_layout(layout, None);
            }
            device.destroy_descriptor_pool(self.descriptor_pool, None);
            device.destroy_sampler(self.sampler, None);
            device.destroy_pipeline(self.graphics_pipeline, None);
            device.destroy_pipeline_layout(self.graphics_pipeline_layout, None);
            device.destroy_shader_module(self.vertex_shader, None);
            device.destroy_shader_module(self.fragment_shader, None);
        }
    }

    pub fn render(&self, ctx: RenderContext) {
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
