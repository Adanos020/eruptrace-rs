use crate::geometry_pass::shaders::{MESH_FRAGMENT_SHADER, MESH_VERTEX_SHADER};
use erupt::{vk, DeviceLoader, ExtendableFrom, SmallVec};
use eruptrace_scene::{mesh::Mesh as SceneMesh, Camera};
use eruptrace_vk::{command, shader::make_shader_module, AllocatedBuffer, PipelineContext, VulkanContext, AllocatedImage};
use itertools::Itertools;
use nalgebra_glm as glm;
use std::ffi::{c_void, CString};
use std::sync::{Arc, RwLock};
use std140::*;
use vk_mem_erupt as vma;

pub mod shaders;

#[repr_std140]
#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    pub position: vec3,
    pub normal: vec3,
    pub texcoord: vec2,
}

#[derive(Clone, Debug)]
pub struct Mesh {
    pub vertex_offset: i32,
    pub first_index: u32,
    pub index_count: u32,
    pub transform: glm::Mat4x4,
    pub material_index: u32,
}

#[repr_std140]
#[derive(Copy, Clone, Debug)]
pub struct MeshMetas {
    pub model_transform: mat4x4,
    pub material_index: uint,
}

#[repr_std140]
#[derive(Copy, Clone, Debug)]
pub struct CameraUniforms {
    pub view_transform: mat4x4,
    pub projection_transform: mat4x4,
}

#[repr_std140]
#[derive(Copy, Clone, Debug)]
pub struct PushConstants {
    mesh_meta_index: uint,
}

#[derive(Clone)]
pub struct GeometryPass {
    meshes: Vec<Mesh>,

    vertex_buffer: AllocatedBuffer<Vertex>,
    index_buffer: AllocatedBuffer<u32>,
    mesh_metas: AllocatedBuffer<MeshMetas>,
    camera_uniforms: AllocatedBuffer<CameraUniforms>,

    output_extent: vk::Extent2D,
    pub out_positions: AllocatedImage,
    pub out_normals: AllocatedImage,
    pub out_texcoords: AllocatedImage,
    pub out_materials: AllocatedImage,

    graphics_pipeline_layout: vk::PipelineLayout,
    graphics_pipeline: vk::Pipeline,

    descriptor_set_layouts: Vec<vk::DescriptorSetLayout>,
    descriptor_pool: vk::DescriptorPool,
    descriptor_sets: SmallVec<vk::DescriptorSet>,
}

impl GeometryPass {
    pub fn new(
        allocator: Arc<RwLock<vma::Allocator>>,
        vk_ctx: VulkanContext,
        pipeline_ctx: PipelineContext,
        camera: &Camera,
        scene_meshes: &[SceneMesh],
    ) -> vma::Result<Self> {
        // Input buffers
        let vertex_buffer = {
            let vertices = scene_meshes
                .iter()
                .flat_map(|m| {
                    m.positions
                        .iter()
                        .zip(m.normals.iter())
                        .zip(m.texcoords.iter())
                        .map(|((position, normal), texcoord)| Vertex {
                            position: eruptrace_vk::std140::vec3(position),
                            normal: eruptrace_vk::std140::vec3(normal),
                            texcoord: eruptrace_vk::std140::vec2(texcoord),
                        })
                })
                .collect_vec();
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

        let index_buffer = {
            let indices = scene_meshes
                .iter()
                .flat_map(|m| m.indices.iter())
                .copied()
                .collect_vec();
            let buffer_info = vk::BufferCreateInfoBuilder::new()
                .usage(vk::BufferUsageFlags::INDEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);
            AllocatedBuffer::with_data(
                allocator.clone(),
                &buffer_info,
                vma::MemoryUsage::CpuToGpu,
                &indices,
            )
        };

        let mut vertices_offset = 0;
        let mut indices_offset = 0;
        let mut meshes = Vec::with_capacity(scene_meshes.len());
        for m in scene_meshes.iter() {
            meshes.push(Mesh {
                vertex_offset: vertices_offset,
                first_index: indices_offset,
                index_count: m.indices.len() as u32,
                transform: m.transform,
                material_index: m.material_index,
            });
            vertices_offset += m.positions.len() as i32;
            indices_offset += m.indices.len() as u32;
        }

        let storage_buffer_info = vk::BufferCreateInfoBuilder::new()
            .usage(vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let uniform_buffer_info = vk::BufferCreateInfoBuilder::new()
            .usage(vk::BufferUsageFlags::UNIFORM_BUFFER | vk::BufferUsageFlags::TRANSFER_DST)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let mesh_metas = {
            let uniforms = meshes
                .iter()
                .map(|m| MeshMetas {
                    model_transform: eruptrace_vk::std140::mat4x4(&m.transform),
                    material_index: uint(m.material_index),
                })
                .collect_vec();
            AllocatedBuffer::with_data(
                allocator.clone(),
                &storage_buffer_info,
                vma::MemoryUsage::CpuToGpu,
                &uniforms,
            )
        };

        let camera_uniforms = {
            let view = glm::look_at(&camera.position, &camera.look_at, &camera.up);
            let proj = glm::perspective(
                camera.img_size[0] as f32 / camera.img_size[1] as f32,
                camera.vertical_fov.to_radians(),
                0.0001,
                100.0,
            );
            let uniforms = vec![CameraUniforms {
                view_transform: eruptrace_vk::std140::mat4x4(&view),
                projection_transform: eruptrace_vk::std140::mat4x4(&proj),
            }];
            AllocatedBuffer::with_data(allocator.clone(), &uniform_buffer_info, vma::MemoryUsage::CpuToGpu, &uniforms)
        };

        // Output images
        let make_gbuffer = |format| AllocatedImage::color_attachment(
            vk_ctx.clone(),
            allocator.clone(),
            format,
            vk::Extent3D {
                width: 1,
                height: 1,
                depth: 1,
            },
            vk::ImageViewType::_2D,
            1,
            1,
        );

        let out_positions = make_gbuffer(vk::Format::R32G32B32A32_SFLOAT);
        let out_normals = make_gbuffer(vk::Format::R32G32B32A32_SFLOAT);
        let out_texcoords = make_gbuffer(vk::Format::R32G32_SFLOAT);
        let out_materials = make_gbuffer(vk::Format::R32_UINT);

        // Descriptor sets
        let descriptor_pool = {
            let sizes = vec![
                vk::DescriptorPoolSizeBuilder::new()
                    ._type(vk::DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(10),
                vk::DescriptorPoolSizeBuilder::new()
                    ._type(vk::DescriptorType::UNIFORM_BUFFER)
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

        let descriptor_set_layouts = {
            let mesh_uniforms_bindings = vec![vk::DescriptorSetLayoutBindingBuilder::new()
                .binding(0)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)];
            let camera_uniforms_bindings = vec![vk::DescriptorSetLayoutBindingBuilder::new()
                .binding(0)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .stage_flags(vk::ShaderStageFlags::VERTEX)];

            let mesh_uniforms_create_info =
                vk::DescriptorSetLayoutCreateInfoBuilder::new().bindings(&mesh_uniforms_bindings);
            let camera_uniforms_create_info =
                vk::DescriptorSetLayoutCreateInfoBuilder::new().bindings(&camera_uniforms_bindings);

            unsafe {
                vec![
                    vk_ctx
                        .device
                        .create_descriptor_set_layout(&mesh_uniforms_create_info, None)
                        .expect("Cannot create descriptor set layout"),
                    vk_ctx
                        .device
                        .create_descriptor_set_layout(&camera_uniforms_create_info, None)
                        .expect("Cannot create descriptor set layout"),
                ]
            }
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

        let storage_buffer_infos = vec![vk::DescriptorBufferInfoBuilder::new()
            .buffer(mesh_metas.buffer)
            .range(vk::WHOLE_SIZE)];

        let uniform_buffer_infos = vec![vk::DescriptorBufferInfoBuilder::new()
            .buffer(camera_uniforms.buffer)
            .range(vk::WHOLE_SIZE)];

        let descriptor_set_writes = vec![
            vk::WriteDescriptorSetBuilder::new()
                .dst_binding(0)
                .dst_set(descriptor_sets[0])
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(&storage_buffer_infos),
            vk::WriteDescriptorSetBuilder::new()
                .dst_binding(0)
                .dst_set(descriptor_sets[1])
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(&uniform_buffer_infos),
        ];

        unsafe {
            vk_ctx
                .device
                .update_descriptor_sets(&descriptor_set_writes, &[]);
        }

        // Graphics pipeline
        let vertex_shader = make_shader_module(&vk_ctx.device, MESH_VERTEX_SHADER);
        let fragment_shader = make_shader_module(&vk_ctx.device, MESH_FRAGMENT_SHADER);

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

        let push_constants = vec![vk::PushConstantRangeBuilder::new()
            .offset(0)
            .size(std::mem::size_of::<PushConstants>() as u32)
            .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)];

        let graphics_pipeline_layout = {
            let create_info = vk::PipelineLayoutCreateInfoBuilder::new()
                .set_layouts(&descriptor_set_layouts)
                .push_constant_ranges(&push_constants);
            unsafe {
                vk_ctx
                    .device
                    .create_pipeline_layout(&create_info, None)
                    .expect("Cannot create graphics pipeline layout")
            }
        };

        let input_assembly = vk::PipelineInputAssemblyStateCreateInfoBuilder::new()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
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
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
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

        let binding_descriptions = vec![vk::VertexInputBindingDescriptionBuilder::new()
            .binding(0)
            .input_rate(vk::VertexInputRate::VERTEX)
            .stride(std::mem::size_of::<Vertex>() as u32)];
        let attribute_descriptions = vec![
            // position
            vk::VertexInputAttributeDescriptionBuilder::new()
                .binding(0)
                .location(0)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(0),
            // normal
            vk::VertexInputAttributeDescriptionBuilder::new()
                .binding(0)
                .location(1)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(std::mem::size_of::<vec3>() as u32),
            // texCoord
            vk::VertexInputAttributeDescriptionBuilder::new()
                .binding(0)
                .location(2)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(std::mem::size_of::<[vec3; 2]>() as u32),
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
            meshes,
            vertex_buffer,
            index_buffer,
            mesh_metas,
            camera_uniforms,
            output_extent: vk::Extent2D {
                width: camera.img_size[0],
                height: camera.img_size[1],
            },
            out_positions,
            out_normals,
            out_texcoords,
            out_materials,
            descriptor_set_layouts,
            descriptor_pool,
            descriptor_sets,
            graphics_pipeline_layout,
            graphics_pipeline,
        })
    }

    pub fn destroy(&self, device: &DeviceLoader) {
        self.vertex_buffer.destroy();
        self.index_buffer.destroy();
        self.camera_uniforms.destroy();
        self.mesh_metas.destroy();
        self.out_positions.destroy(device);
        self.out_normals.destroy(device);
        self.out_texcoords.destroy(device);
        self.out_materials.destroy(device);
        unsafe {
            for &layout in self.descriptor_set_layouts.iter() {
                device.destroy_descriptor_set_layout(layout, None);
            }
            device.destroy_descriptor_pool(self.descriptor_pool, None);
            device.destroy_pipeline(self.graphics_pipeline, None);
            device.destroy_pipeline_layout(self.graphics_pipeline_layout, None);
        }
    }

    pub fn update_camera(&mut self, camera: Camera) {
        let view = glm::look_at(&camera.position, &camera.look_at, &camera.up);
        let mut proj = glm::perspective(
            camera.img_size[0] as f32 / camera.img_size[1] as f32,
            camera.vertical_fov.to_radians(),
            0.001,
            100.0,
        );
        proj[(1, 1)] *= -1.0;
        let data = CameraUniforms {
            view_transform: eruptrace_vk::std140::mat4x4(&view),
            projection_transform: eruptrace_vk::std140::mat4x4(&proj),
        };
        self.camera_uniforms.set_data(&[data]);
    }

    pub fn render(&self, vk_ctx: VulkanContext) {
        let make_attachment = |view| vk::RenderingAttachmentInfoBuilder::new()
            .image_view(view)
            .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .clear_value(vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 0.0],
                },
            })
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .resolve_image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);
        let colour_attachments = vec![
            make_attachment(self.out_positions.view),
            make_attachment(self.out_normals.view),
            make_attachment(self.out_texcoords.view),
            make_attachment(self.out_materials.view),
        ];
        let rendering_info = vk::RenderingInfoBuilder::new()
            .color_attachments(&colour_attachments)
            .layer_count(1)
            .render_area(vk::Rect2D {
                offset: Default::default(),
                extent: self.output_extent,
            });

        command::immediate_submit(vk_ctx, |device, command_buffer| unsafe {
            device.cmd_set_scissor(
                command_buffer,
                0,
                &[vk::Rect2DBuilder::new().extent(self.output_extent)],
            );
            device.cmd_set_viewport(
                command_buffer,
                0,
                &[
                    vk::ViewportBuilder::new()
                        .width(self.output_extent.width as _)
                        .height(self.output_extent.height as _)
                        .min_depth(0.0)
                        .max_depth(1.0),
                ],
            );
            device.cmd_begin_rendering(command_buffer, &rendering_info);
            device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.graphics_pipeline,
            );
            device.cmd_bind_vertex_buffers(
                command_buffer,
                0,
                &[self.vertex_buffer.buffer],
                &[0],
            );
            device.cmd_bind_index_buffer(
                command_buffer,
                self.index_buffer.buffer,
                0,
                vk::IndexType::UINT32,
            );
            device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.graphics_pipeline_layout,
                0,
                &self.descriptor_sets,
                &[],
            );
            for (i, mesh) in self.meshes.iter().enumerate() {
                let push_constants = PushConstants {
                    mesh_meta_index: uint(i as u32),
                };
                device.cmd_push_constants(
                    command_buffer,
                    self.graphics_pipeline_layout,
                    vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                    0,
                    std::mem::size_of::<PushConstants>() as u32,
                    &push_constants as *const PushConstants as *const c_void,
                );
                device.cmd_draw_indexed(
                    command_buffer,
                    mesh.index_count,
                    1,
                    mesh.first_index,
                    mesh.vertex_offset,
                    0,
                );
            }
            device.cmd_end_rendering(command_buffer);
        });
    }
}
