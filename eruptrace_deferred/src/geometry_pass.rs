use std::ffi::c_void;

use erupt::{vk, DeviceLoader};
use eruptrace_scene::{mesh::Mesh as SceneMesh, Camera};
use eruptrace_vk::{
    command,
    pipeline::{
        ColorAttachmentInfo,
        DescriptorBindingCreateInfo,
        DescriptorSetCreateInfo,
        GraphicsPipelineCreateInfo,
        Pipeline,
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

use crate::{
    gbuffers::GBuffers,
    shaders::{MESH_FRAGMENT_SHADER, MESH_VERTEX_SHADER},
};

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

    output_extent: vk::Extent2D,
    pub gbuffers:  GBuffers,
    depth_buffer:  AllocatedImage,

    graphics_pipeline: Pipeline,
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
        let output_extent = vk::Extent3D { width: camera.sqrt_samples, height: camera.sqrt_samples, depth: 1 };

        let make_gbuffer =
            |format| AllocatedImage::gbuffer(vk_ctx.clone(), format, output_extent, vk::ImageViewType::_2D, 1, 1);

        let out_positions = make_gbuffer(vk::Format::R32G32B32A32_SFLOAT);
        let out_normals = make_gbuffer(vk::Format::R32G32B32A32_SFLOAT);
        let out_materials = make_gbuffer(vk::Format::R32G32B32A32_SFLOAT);

        let depth_buffer = AllocatedImage::depth_buffer(vk_ctx.clone(), output_extent);

        // Graphics pipeline
        let graphics_pipeline = Pipeline::graphics(vk_ctx.clone(), GraphicsPipelineCreateInfo {
            vertex_shader:           MESH_VERTEX_SHADER,
            fragment_shader:         MESH_FRAGMENT_SHADER,
            color_attachment_infos:  vec![
                // positions
                ColorAttachmentInfo {
                    format:           vk::Format::R32G32B32A32_SFLOAT,
                    color_write_mask: vk::ColorComponentFlags::all(),
                    blend_enable:     false,
                },
                // normals
                ColorAttachmentInfo {
                    format:           vk::Format::R32G32B32A32_SFLOAT,
                    color_write_mask: vk::ColorComponentFlags::all(),
                    blend_enable:     false,
                },
                // materials
                ColorAttachmentInfo {
                    format:           vk::Format::R32G32B32A32_SFLOAT,
                    color_write_mask: vk::ColorComponentFlags::all(),
                    blend_enable:     false,
                },
            ],
            colour_blending_info:    vk::PipelineColorBlendStateCreateInfoBuilder::new().logic_op_enable(false),
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
            enable_depth_testing:    true,
        });

        Ok(Self {
            meshes,
            vertex_buffer,
            index_buffer,
            mesh_metas,
            camera_uniforms,
            output_extent: camera.image_extent_2d(),
            gbuffers: GBuffers { out_positions, out_normals, out_materials },
            depth_buffer,
            graphics_pipeline,
        })
    }

    pub fn destroy(&self, device: &DeviceLoader) {
        self.vertex_buffer.destroy();
        self.index_buffer.destroy();
        self.camera_uniforms.destroy();
        self.mesh_metas.destroy();
        self.gbuffers.destroy(device);
        self.depth_buffer.destroy(device);
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

        self.output_extent = vk::Extent2D {
            width:  camera.img_size[0] * camera.sqrt_samples,
            height: camera.img_size[1] * camera.sqrt_samples,
        };
        let gbuffer_extent =
            vk::Extent3D { width: self.output_extent.width, height: self.output_extent.height, depth: 1 };

        let make_color_attachment =
            |format| AllocatedImage::gbuffer(vk_ctx.clone(), format, gbuffer_extent, vk::ImageViewType::_2D, 1, 1);

        self.gbuffers.out_positions.destroy(&vk_ctx.device);
        self.gbuffers.out_normals.destroy(&vk_ctx.device);
        self.gbuffers.out_materials.destroy(&vk_ctx.device);
        self.depth_buffer.destroy(&vk_ctx.device);

        self.gbuffers.out_positions = make_color_attachment(vk::Format::R32G32B32A32_SFLOAT);
        self.gbuffers.out_normals = make_color_attachment(vk::Format::R32G32B32A32_SFLOAT);
        self.gbuffers.out_materials = make_color_attachment(vk::Format::R32G32B32A32_SFLOAT);
        self.depth_buffer = AllocatedImage::depth_buffer(vk_ctx.clone(), gbuffer_extent);
    }

    pub fn render(&self, vk_ctx: VulkanContext) {
        let colour_attachments = self.gbuffers.create_colour_attachment_infos();
        let depth_attachment = vk::RenderingAttachmentInfoBuilder::new()
            .image_view(self.depth_buffer.view)
            .image_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
            .clear_value(vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue { depth: 1.0, ..Default::default() },
            })
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE);
        let rendering_info = vk::RenderingInfoBuilder::new()
            .color_attachments(&colour_attachments)
            .depth_attachment(&depth_attachment)
            .layer_count(1)
            .render_area(vk::Rect2D { offset: Default::default(), extent: self.output_extent });

        command::immediate_submit(vk_ctx, |device, command_buffer| unsafe {
            command::set_scissor_and_viewport(device, command_buffer, self.output_extent);

            device.cmd_pipeline_barrier2(
                command_buffer,
                &vk::DependencyInfoBuilder::new().image_memory_barriers({
                    let barrier = vk::ImageMemoryBarrier2Builder::new()
                        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                        .src_stage_mask(vk::PipelineStageFlags2::FRAGMENT_SHADER)
                        .dst_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
                        .src_access_mask(vk::AccessFlags2::SHADER_SAMPLED_READ)
                        .dst_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
                        .old_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                        .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);
                    &[
                        barrier
                            .image(self.gbuffers.out_positions.image)
                            .subresource_range(self.gbuffers.out_positions.subresource_range),
                        barrier
                            .image(self.gbuffers.out_normals.image)
                            .subresource_range(self.gbuffers.out_normals.subresource_range),
                        barrier
                            .image(self.gbuffers.out_materials.image)
                            .subresource_range(self.gbuffers.out_materials.subresource_range),
                    ]
                }),
            );

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

            device.cmd_pipeline_barrier2(
                command_buffer,
                &vk::DependencyInfoBuilder::new().image_memory_barriers({
                    let barrier = vk::ImageMemoryBarrier2Builder::new()
                        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                        .src_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
                        .dst_stage_mask(vk::PipelineStageFlags2::FRAGMENT_SHADER)
                        .src_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
                        .dst_access_mask(vk::AccessFlags2::SHADER_SAMPLED_READ)
                        .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                        .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
                    &[
                        barrier
                            .image(self.gbuffers.out_positions.image)
                            .subresource_range(self.gbuffers.out_positions.subresource_range),
                        barrier
                            .image(self.gbuffers.out_normals.image)
                            .subresource_range(self.gbuffers.out_normals.subresource_range),
                        barrier
                            .image(self.gbuffers.out_materials.image)
                            .subresource_range(self.gbuffers.out_materials.subresource_range),
                    ]
                }),
            );
        });
    }
}
