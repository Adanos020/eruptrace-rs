use egui::{epaint::Vertex, TextureId};
use erupt::vk;
use eruptrace_vk::{
    pipeline::{
        ColorAttachmentInfo,
        DescriptorBindingCreateInfo,
        DescriptorSetCreateInfo,
        GraphicsPipelineCreateInfo,
        Pipeline,
        RasterisationStateInfo,
        SamplerCreateInfo,
    },
    VulkanContext,
};

use crate::{
    gui::GuiPushConstants,
    shaders::{GUI_MESH_FRAGMENT_SHADER, GUI_MESH_VERTEX_SHADER},
};

#[derive(Clone)]
pub struct Mesh {
    pub vertex_offset:     i32,
    pub first_index:       u32,
    pub index_count:       u32,
    pub graphics_pipeline: Pipeline,
    pub scissor:           vk::Rect2D,
    pub texture_id:        TextureId,
}

impl Mesh {
    pub fn new(
        vk_ctx: VulkanContext,
        vertex_offset: i32,
        first_index: u32,
        index_count: u32,
        surface_format: vk::SurfaceFormatKHR,
        image_view: vk::ImageView,
        scissor: vk::Rect2D,
        texture_id: TextureId,
    ) -> Self {
        let graphics_pipeline = Pipeline::graphics(vk_ctx, GraphicsPipelineCreateInfo {
            vertex_shader:           GUI_MESH_VERTEX_SHADER,
            fragment_shader:         GUI_MESH_FRAGMENT_SHADER,
            color_attachment_infos:  vec![ColorAttachmentInfo {
                format:           surface_format.format,
                color_write_mask: vk::ColorComponentFlags::all(),
            }],
            push_constant_ranges:    vec![vk::PushConstantRangeBuilder::new()
                .stage_flags(vk::ShaderStageFlags::VERTEX)
                .offset(0)
                .size(std::mem::size_of::<GuiPushConstants>() as u32)],
            input_assembly:          vk::PipelineInputAssemblyStateCreateInfoBuilder::new()
                .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
                .primitive_restart_enable(false),
            vertex_input_bindings:   vec![vk::VertexInputBindingDescriptionBuilder::new()
                .binding(0)
                .input_rate(vk::VertexInputRate::VERTEX)
                .stride(std::mem::size_of::<Vertex>() as u32)],
            vertex_input_attributes: vec![
                // pos
                vk::VertexInputAttributeDescriptionBuilder::new()
                    .binding(0)
                    .location(0)
                    .format(vk::Format::R32G32_SFLOAT)
                    .offset(0),
                // uv
                vk::VertexInputAttributeDescriptionBuilder::new()
                    .binding(0)
                    .location(1)
                    .format(vk::Format::R32G32_SFLOAT)
                    .offset(std::mem::size_of::<[f32; 2]>() as u32),
                // color
                vk::VertexInputAttributeDescriptionBuilder::new()
                    .binding(0)
                    .location(2)
                    .format(vk::Format::R8G8B8A8_UNORM)
                    .offset(std::mem::size_of::<[f32; 4]>() as u32),
            ],
            rasterisation_state:     RasterisationStateInfo {
                cull_mode:  vk::CullModeFlags::NONE,
                front_face: vk::FrontFace::COUNTER_CLOCKWISE,
            },
            descriptor_sets_infos:   vec![DescriptorSetCreateInfo {
                descriptor_infos: vec![DescriptorBindingCreateInfo::image(
                    vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    vk::ShaderStageFlags::FRAGMENT,
                    vk::DescriptorImageInfoBuilder::new()
                        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                        .image_view(image_view),
                    0,
                )],
            }],
            sampler_infos:           vec![SamplerCreateInfo {
                address_mode: vk::SamplerAddressMode::CLAMP_TO_EDGE,
                filter:       vk::Filter::LINEAR,
            }],
            enable_depth_testing:    false,
        });

        Self { vertex_offset, first_index, index_count, graphics_pipeline, scissor, texture_id }
    }
}
