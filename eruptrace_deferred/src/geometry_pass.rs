use std::ffi::c_void;

use erupt::{vk, DeviceLoader};
use eruptrace_scene::{mesh::Mesh as SceneMesh, Camera};
use eruptrace_vk::{
    command,
    pipeline::{
        ColorAttachmentInfo,
        DescriptorBindingCreateInfo,
        DescriptorSetCreateInfo,
        GraphicsPipeline,
        GraphicsPipelineCreateInfo,
        RasterisationStateInfo,
    },
    AllocatedBuffer,
    AllocatedImage,
    VulkanContext,
};
use itertools::Itertools;
use nalgebra_glm as glm;
use std140::repr_std140;
use vk_mem_erupt as vma;

use crate::shaders::{MESH_FRAGMENT_SHADER, MESH_VERTEX_SHADER};

#[repr_std140]
#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    pub position: std140::vec3,
    pub normal:   std140::vec3,
    pub texcoord: std140::vec2,
}

#[derive(Clone, Debug)]
pub struct Mesh {
    pub vertex_offset:  i32,
    pub first_index:    u32,
    pub index_count:    u32,
    pub transform:      glm::Mat4x4,
    pub material_index: u32,
}

#[repr_std140]
#[derive(Copy, Clone, Debug)]
pub struct MeshMetas {
    pub model_transform: std140::mat4x4,
    pub material_index:  std140::uint,
}

#[repr_std140]
#[derive(Copy, Clone, Debug)]
pub struct CameraUniforms {
    pub view_transform:       std140::mat4x4,
    pub projection_transform: std140::mat4x4,
}

#[repr_std140]
#[derive(Copy, Clone, Debug)]
pub struct PushConstants {
    mesh_meta_index: std140::uint,
}

#[derive(Clone)]
pub struct GeometryPass {
    meshes: Vec<Mesh>,

    vertex_buffer:   AllocatedBuffer<Vertex>,
    index_buffer:    AllocatedBuffer<u32>,
    mesh_metas:      AllocatedBuffer<MeshMetas>,
    camera_uniforms: AllocatedBuffer<CameraUniforms>,

    output_extent:     vk::Extent2D,
    pub out_positions: AllocatedImage,
    pub out_normals:   AllocatedImage,
    pub out_texcoords: AllocatedImage,

    graphics_pipeline: GraphicsPipeline,
}

impl GeometryPass {
    pub fn new(vk_ctx: VulkanContext, camera: &Camera, scene_meshes: Vec<SceneMesh>) -> vma::Result<Self> {
        // Input buffers
        let vertex_buffer = {
            let vertices = scene_meshes
                .iter()
                .flat_map(|m| {
                    m.positions.iter().zip(m.normals.iter()).zip(m.texcoords.iter()).map(
                        |((position, normal), texcoord)| Vertex {
                            position: eruptrace_vk::std140::vec3(position),
                            normal:   eruptrace_vk::std140::vec3(normal),
                            texcoord: eruptrace_vk::std140::vec2(texcoord),
                        },
                    )
                })
                .collect_vec();
            let buffer_info = vk::BufferCreateInfoBuilder::new()
                .usage(vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);
            AllocatedBuffer::with_data(vk_ctx.allocator.clone(), &buffer_info, vma::MemoryUsage::CpuToGpu, &vertices)
        };

        let index_buffer = {
            let indices = scene_meshes.iter().flat_map(|m| m.indices.iter()).copied().collect_vec();
            let buffer_info = vk::BufferCreateInfoBuilder::new()
                .usage(vk::BufferUsageFlags::INDEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);
            AllocatedBuffer::with_data(vk_ctx.allocator.clone(), &buffer_info, vma::MemoryUsage::CpuToGpu, &indices)
        };

        let mut vertices_offset = 0;
        let mut indices_offset = 0;
        let mut meshes = Vec::with_capacity(scene_meshes.len());
        for m in scene_meshes.iter() {
            meshes.push(Mesh {
                vertex_offset:  vertices_offset,
                first_index:    indices_offset,
                index_count:    m.indices.len() as u32,
                transform:      m.transform,
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
                    material_index:  std140::uint(m.material_index),
                })
                .collect_vec();
            AllocatedBuffer::with_data(
                vk_ctx.allocator.clone(),
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
                view_transform:       eruptrace_vk::std140::mat4x4(&view),
                projection_transform: eruptrace_vk::std140::mat4x4(&proj),
            }];
            AllocatedBuffer::with_data(
                vk_ctx.allocator.clone(),
                &uniform_buffer_info,
                vma::MemoryUsage::CpuToGpu,
                &uniforms,
            )
        };

        // Output images
        let make_gbuffer = |format| {
            AllocatedImage::color_attachment(
                vk_ctx.clone(),
                format,
                vk::Extent3D { width: 1, height: 1, depth: 1 },
                vk::ImageViewType::_2D,
                1,
                1,
            )
        };

        let out_positions = make_gbuffer(vk::Format::R32G32B32A32_SFLOAT);
        let out_normals = make_gbuffer(vk::Format::R32G32B32A32_SFLOAT);
        let out_texcoords = make_gbuffer(vk::Format::R32G32B32A32_SFLOAT);

        // Graphics pipeline
        let graphics_pipeline = GraphicsPipeline::new(vk_ctx.clone(), GraphicsPipelineCreateInfo {
            vertex_shader:           MESH_VERTEX_SHADER,
            fragment_shader:         MESH_FRAGMENT_SHADER,
            color_attachment_infos:  vec![
                // positions
                ColorAttachmentInfo {
                    format:           vk::Format::R32G32B32A32_SFLOAT,
                    color_write_mask: vk::ColorComponentFlags::all(),
                },
                // normals
                ColorAttachmentInfo {
                    format:           vk::Format::R32G32B32A32_SFLOAT,
                    color_write_mask: vk::ColorComponentFlags::all(),
                },
                // materials
                ColorAttachmentInfo {
                    format:           vk::Format::R32G32B32A32_SFLOAT,
                    color_write_mask: vk::ColorComponentFlags::all(),
                },
            ],
            push_constant_ranges:    vec![vk::PushConstantRangeBuilder::new()
                .offset(0)
                .size(std::mem::size_of::<PushConstants>() as u32)
                .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)],
            input_assembly:          vk::PipelineInputAssemblyStateCreateInfoBuilder::new()
                .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
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
                    .format(vk::Format::R32G32B32_SFLOAT)
                    .offset(0),
                // normal
                vk::VertexInputAttributeDescriptionBuilder::new()
                    .binding(0)
                    .location(1)
                    .format(vk::Format::R32G32B32_SFLOAT)
                    .offset(std::mem::size_of::<std140::vec3>() as u32),
                // texCoord
                vk::VertexInputAttributeDescriptionBuilder::new()
                    .binding(0)
                    .location(2)
                    .format(vk::Format::R32G32_SFLOAT)
                    .offset(std::mem::size_of::<[std140::vec3; 2]>() as u32),
            ],
            rasterisation_state:     RasterisationStateInfo {
                cull_mode:  vk::CullModeFlags::BACK,
                front_face: vk::FrontFace::COUNTER_CLOCKWISE,
            },
            descriptor_sets_infos:   vec![
                DescriptorSetCreateInfo {
                    descriptor_infos: vec![DescriptorBindingCreateInfo::buffer(
                        vk::DescriptorType::STORAGE_BUFFER,
                        vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                        vk::DescriptorBufferInfoBuilder::new().buffer(mesh_metas.buffer).range(vk::WHOLE_SIZE),
                    )],
                },
                DescriptorSetCreateInfo {
                    descriptor_infos: vec![DescriptorBindingCreateInfo::buffer(
                        vk::DescriptorType::UNIFORM_BUFFER,
                        vk::ShaderStageFlags::VERTEX,
                        vk::DescriptorBufferInfoBuilder::new().buffer(camera_uniforms.buffer).range(vk::WHOLE_SIZE),
                    )],
                },
            ],
            sampler_infos:           vec![],
        });

        Ok(Self {
            meshes,
            vertex_buffer,
            index_buffer,
            mesh_metas,
            camera_uniforms,
            output_extent: vk::Extent2D { width: camera.img_size[0], height: camera.img_size[1] },
            out_positions,
            out_normals,
            out_texcoords,
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
        self.graphics_pipeline.destroy(device);
    }

    pub fn update_camera(&mut self, vk_ctx: VulkanContext, camera: Camera) {
        let view = glm::look_at(&camera.position, &camera.look_at, &camera.up);
        let mut proj = glm::perspective(
            camera.img_size[0] as f32 / camera.img_size[1] as f32,
            camera.vertical_fov.to_radians(),
            0.001,
            100.0,
        );
        proj[(1, 1)] *= -1.0;
        let data = CameraUniforms {
            view_transform:       eruptrace_vk::std140::mat4x4(&view),
            projection_transform: eruptrace_vk::std140::mat4x4(&proj),
        };
        self.camera_uniforms.set_data(&[data]);

        let make_color_attachment = |format| {
            AllocatedImage::color_attachment(
                vk_ctx.clone(),
                format,
                vk::Extent3D { width: camera.img_size[0], height: camera.img_size[1], depth: 1 },
                vk::ImageViewType::_2D,
                1,
                1,
            )
        };

        self.out_positions.destroy(&vk_ctx.device);
        self.out_normals.destroy(&vk_ctx.device);
        self.out_texcoords.destroy(&vk_ctx.device);

        self.out_positions = make_color_attachment(vk::Format::R32G32B32A32_SFLOAT);
        self.out_normals = make_color_attachment(vk::Format::R32G32B32A32_SFLOAT);
        self.out_texcoords = make_color_attachment(vk::Format::R32G32B32A32_SFLOAT);

        self.output_extent = vk::Extent2D { width: camera.img_size[0], height: camera.img_size[1] };
    }

    pub fn render(&self, vk_ctx: VulkanContext) {
        let make_attachment_info = |view| {
            vk::RenderingAttachmentInfoBuilder::new()
                .image_view(view)
                .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .clear_value(vk::ClearValue { color: vk::ClearColorValue { float32: [0.0, 0.0, 0.0, 0.0] } })
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .resolve_image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        };
        let colour_attachments = vec![
            make_attachment_info(self.out_positions.view),
            make_attachment_info(self.out_normals.view),
            make_attachment_info(self.out_texcoords.view),
        ];
        let rendering_info = vk::RenderingInfoBuilder::new()
            .color_attachments(&colour_attachments)
            .layer_count(1)
            .render_area(vk::Rect2D { offset: Default::default(), extent: self.output_extent });

        command::immediate_submit(vk_ctx, |device, command_buffer| unsafe {
            device.cmd_set_scissor(command_buffer, 0, &[vk::Rect2DBuilder::new().extent(self.output_extent)]);
            device.cmd_set_viewport(command_buffer, 0, &[vk::ViewportBuilder::new()
                .width(self.output_extent.width as _)
                .height(self.output_extent.height as _)
                .min_depth(0.0)
                .max_depth(1.0)]);
            device.cmd_begin_rendering(command_buffer, &rendering_info);
            device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, self.graphics_pipeline.pipeline);
            device.cmd_bind_vertex_buffers(command_buffer, 0, &[self.vertex_buffer.buffer], &[0]);
            device.cmd_bind_index_buffer(command_buffer, self.index_buffer.buffer, 0, vk::IndexType::UINT32);
            device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.graphics_pipeline.layout,
                0,
                &self.graphics_pipeline.descriptor_sets,
                &[],
            );
            for (i, mesh) in self.meshes.iter().enumerate() {
                let push_constants = PushConstants { mesh_meta_index: std140::uint(i as u32) };
                device.cmd_push_constants(
                    command_buffer,
                    self.graphics_pipeline.layout,
                    vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                    0,
                    std::mem::size_of::<PushConstants>() as u32,
                    &push_constants as *const PushConstants as *const c_void,
                );
                device.cmd_draw_indexed(command_buffer, mesh.index_count, 1, mesh.first_index, mesh.vertex_offset, 0);
            }
            device.cmd_end_rendering(command_buffer);
        });
    }
}
