use std::ffi::CString;

use erupt::{vk, DeviceLoader, ExtendableFrom, SmallVec};
use itertools::Itertools;

use crate::{shader::make_shader_module, VulkanContext};

#[derive(Clone, Debug)]
pub struct GraphicsPipeline {
    pub samplers:               Vec<vk::Sampler>,
    pub descriptor_set_layouts: Vec<vk::DescriptorSetLayout>,
    pub descriptor_pool:        vk::DescriptorPool,
    pub descriptor_sets:        SmallVec<vk::DescriptorSet>,
    pub layout:                 vk::PipelineLayout,
    pub pipeline:               vk::Pipeline,
}

#[derive(Clone, Debug)]
pub struct GraphicsPipelineCreateInfo<'a> {
    pub vertex_shader:           &'static [u8],
    pub fragment_shader:         &'static [u8],
    pub color_attachment_infos:  Vec<ColorAttachmentInfo>,
    pub push_constant_ranges:    Vec<vk::PushConstantRangeBuilder<'a>>,
    pub input_assembly:          vk::PipelineInputAssemblyStateCreateInfoBuilder<'a>,
    pub vertex_input_bindings:   Vec<vk::VertexInputBindingDescriptionBuilder<'a>>,
    pub vertex_input_attributes: Vec<vk::VertexInputAttributeDescriptionBuilder<'a>>,
    pub rasterisation_state:     RasterisationStateInfo,
    pub descriptor_sets_infos:   Vec<DescriptorSetCreateInfo<'a>>,
    pub sampler_infos:           Vec<SamplerCreateInfo>,
}

#[derive(Copy, Clone, Debug)]
pub struct RasterisationStateInfo {
    pub cull_mode:  vk::CullModeFlags,
    pub front_face: vk::FrontFace,
}

#[derive(Clone, Debug)]
pub struct DescriptorSetCreateInfo<'a> {
    pub descriptor_infos: Vec<DescriptorBindingCreateInfo<'a>>,
}

#[derive(Copy, Clone, Debug)]
pub struct DescriptorBindingCreateInfo<'a> {
    pub descriptor_type:    vk::DescriptorType,
    pub shader_stage_flags: vk::ShaderStageFlags,
    pub buffer_info:        Option<vk::DescriptorBufferInfoBuilder<'a>>,
    pub image_info:         Option<vk::DescriptorImageInfoBuilder<'a>>,
    pub sampler_index:      Option<usize>,
}

#[derive(Clone, Debug)]
pub struct ColorAttachmentInfo {
    pub format:           vk::Format,
    pub color_write_mask: vk::ColorComponentFlags,
}

#[derive(Copy, Clone, Debug)]
pub struct SamplerCreateInfo {
    pub address_mode: vk::SamplerAddressMode,
    pub filter:       vk::Filter,
}

impl GraphicsPipeline {
    pub fn new(vk_ctx: VulkanContext, create_info: GraphicsPipelineCreateInfo) -> GraphicsPipeline {
        let samplers = create_info
            .sampler_infos
            .into_iter()
            .map(|info| {
                let create_info = vk::SamplerCreateInfoBuilder::new()
                    .address_mode_u(info.address_mode)
                    .address_mode_v(info.address_mode)
                    .address_mode_w(info.address_mode)
                    .min_filter(info.filter)
                    .mag_filter(info.filter);
                unsafe { vk_ctx.device.create_sampler(&create_info, None).expect("Cannot create sampler") }
            })
            .collect_vec();

        let descriptor_set_layouts = create_info
            .descriptor_sets_infos
            .iter()
            .map(|set_info| {
                let infos = set_info
                    .descriptor_infos
                    .iter()
                    .enumerate()
                    .map(|(binding, bind_info)| {
                        vk::DescriptorSetLayoutBindingBuilder::new()
                            .binding(binding as u32)
                            .descriptor_count(1)
                            .descriptor_type(bind_info.descriptor_type)
                            .stage_flags(bind_info.shader_stage_flags)
                    })
                    .collect_vec();
                let create_info = vk::DescriptorSetLayoutCreateInfoBuilder::new().bindings(&infos);
                unsafe {
                    vk_ctx
                        .device
                        .create_descriptor_set_layout(&create_info, None)
                        .expect("Cannot create descriptor set layout")
                }
            })
            .collect_vec();

        let descriptor_pool = {
            let sizes = create_info
                .descriptor_sets_infos
                .iter()
                .flat_map(|set_info| set_info.descriptor_infos.iter())
                .map(|bind_info| bind_info.descriptor_type)
                .dedup()
                .map(|descriptor_type| vk::DescriptorPoolSizeBuilder::new()._type(descriptor_type).descriptor_count(10))
                .collect_vec();
            let create_info = vk::DescriptorPoolCreateInfoBuilder::new().max_sets(10).pool_sizes(&sizes);
            unsafe { vk_ctx.device.create_descriptor_pool(&create_info, None).expect("Cannot create descriptor pool") }
        };

        let descriptor_sets = unsafe {
            vk_ctx
                .device
                .allocate_descriptor_sets(
                    &vk::DescriptorSetAllocateInfoBuilder::new()
                        .descriptor_pool(descriptor_pool)
                        .set_layouts(&descriptor_set_layouts),
                )
                .expect("Cannot allocate descriptor sets")
        };

        let mut descriptor_writes_data = Vec::new();
        for (set, set_info) in create_info.descriptor_sets_infos.iter().enumerate() {
            let mut curr_is_image = set_info.descriptor_infos[0].image_info.is_some();
            let mut curr_type = set_info.descriptor_infos[0].descriptor_type;
            let mut buffer_infos = Vec::new();
            let mut image_infos = Vec::new();
            let mut first_binding = 0u32;
            for (i, binding_info) in set_info.descriptor_infos.iter().enumerate() {
                assert_ne!(binding_info.buffer_info.is_some(), binding_info.image_info.is_some());
                assert_eq!(binding_info.image_info.is_some(), binding_info.sampler_index.is_some());

                if curr_type != binding_info.descriptor_type || curr_is_image != binding_info.image_info.is_some() {
                    descriptor_writes_data.push((
                        set,
                        first_binding,
                        curr_type,
                        curr_is_image,
                        buffer_infos,
                        image_infos,
                    ));
                    curr_type = binding_info.descriptor_type;
                    curr_is_image = binding_info.image_info.is_some();
                    buffer_infos = Vec::new();
                    image_infos = Vec::new();
                    first_binding = i as u32;
                }

                if binding_info.buffer_info.is_some() {
                    buffer_infos.push(binding_info.buffer_info.unwrap().into_builder());
                } else {
                    image_infos.push(
                        binding_info
                            .image_info
                            .unwrap()
                            .sampler(samplers[binding_info.sampler_index.unwrap()])
                            .into_builder(),
                    );
                }

                if i == set_info.descriptor_infos.len() - 1 {
                    descriptor_writes_data.push((
                        set,
                        first_binding,
                        curr_type,
                        curr_is_image,
                        buffer_infos,
                        image_infos,
                    ));
                    break; // This is just an easy way to dismiss the "use of moved value" error.
                }
            }
        }

        let descriptor_writes = descriptor_writes_data
            .iter()
            .map(|(set, first_binding, descriptor_type, is_image, buffer_infos, image_infos)| {
                let write = vk::WriteDescriptorSetBuilder::new()
                    .dst_binding(*first_binding)
                    .dst_set(descriptor_sets[*set])
                    .descriptor_type(*descriptor_type);
                if *is_image {
                    write.image_info(image_infos)
                } else {
                    write.buffer_info(buffer_infos)
                }
            })
            .collect_vec();

        unsafe {
            vk_ctx.device.update_descriptor_sets(&descriptor_writes, &[]);
        }

        let vertex_shader = make_shader_module(&vk_ctx.device, create_info.vertex_shader);
        let fragment_shader = make_shader_module(&vk_ctx.device, create_info.fragment_shader);

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

        let layout = {
            let create_info = vk::PipelineLayoutCreateInfoBuilder::new()
                .set_layouts(&descriptor_set_layouts)
                .push_constant_ranges(&create_info.push_constant_ranges);
            unsafe {
                vk_ctx
                    .device
                    .create_pipeline_layout(&create_info, None)
                    .expect("Cannot create graphics pipeline layout")
            }
        };

        let dynamic_pipeline_state = vk::PipelineDynamicStateCreateInfoBuilder::new()
            .dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]);

        let viewport_state = vk::PipelineViewportStateCreateInfoBuilder::new().viewport_count(1).scissor_count(1);

        let multisample_state = vk::PipelineMultisampleStateCreateInfoBuilder::new()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlagBits::_1);

        let color_attachment_formats = create_info.color_attachment_infos.iter().map(|info| info.format).collect_vec();
        let mut pipeline_rendering_info =
            vk::PipelineRenderingCreateInfoBuilder::new().color_attachment_formats(&color_attachment_formats);

        let colour_blend_attachments = create_info
            .color_attachment_infos
            .iter()
            .map(|info| {
                vk::PipelineColorBlendAttachmentStateBuilder::new()
                    .color_write_mask(info.color_write_mask)
                    .blend_enable(false)
            })
            .collect_vec();
        let colour_blending_info = vk::PipelineColorBlendStateCreateInfoBuilder::new()
            .logic_op_enable(false)
            .attachments(&colour_blend_attachments);

        let vertex_input = vk::PipelineVertexInputStateCreateInfoBuilder::new()
            .vertex_binding_descriptions(&create_info.vertex_input_bindings)
            .vertex_attribute_descriptions(&create_info.vertex_input_attributes);

        let rasterisation_state = vk::PipelineRasterizationStateCreateInfoBuilder::new()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(create_info.rasterisation_state.cull_mode)
            .front_face(create_info.rasterisation_state.front_face)
            .depth_clamp_enable(false);

        let pipeline_infos = vec![vk::GraphicsPipelineCreateInfoBuilder::new()
            .vertex_input_state(&vertex_input)
            .color_blend_state(&colour_blending_info)
            .multisample_state(&multisample_state)
            .stages(&shader_stages)
            .layout(layout)
            .rasterization_state(&rasterisation_state)
            .dynamic_state(&dynamic_pipeline_state)
            .viewport_state(&viewport_state)
            .input_assembly_state(&create_info.input_assembly)
            .extend_from(&mut pipeline_rendering_info)];

        let pipeline = unsafe {
            vk_ctx
                .device
                .create_graphics_pipelines(vk::PipelineCache::null(), &pipeline_infos, None)
                .expect("Cannot create graphics pipeline")[0]
        };

        unsafe {
            vk_ctx.device.destroy_shader_module(vertex_shader, None);
            vk_ctx.device.destroy_shader_module(fragment_shader, None);
        }

        GraphicsPipeline { samplers, descriptor_set_layouts, descriptor_pool, descriptor_sets, layout, pipeline }
    }

    pub fn destroy(&self, device: &DeviceLoader) {
        unsafe {
            for &layout in self.descriptor_set_layouts.iter() {
                device.destroy_descriptor_set_layout(layout, None);
            }
            device.destroy_descriptor_pool(self.descriptor_pool, None);
            for &sampler in self.samplers.iter() {
                device.destroy_sampler(sampler, None);
            }
            device.destroy_pipeline(self.pipeline, None);
            device.destroy_pipeline_layout(self.layout, None);
        }
    }
}
