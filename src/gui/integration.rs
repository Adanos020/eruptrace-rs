use std::{borrow::Borrow, ffi::c_void};

use egui::{
    epaint::{ahash::AHashMap, Vertex},
    ClippedMesh,
    ImageData,
    TextureId,
    TexturesDelta,
};
use erupt::{vk, DeviceLoader};
use eruptrace_vk::{
    contexts::RenderContext,
    push_constants::GuiPushConstants,
    AllocatedBuffer,
    AllocatedImage,
    VulkanContext,
};
use itertools::Itertools;
use nalgebra_glm as glm;
use vk_mem_erupt as vma;

use crate::gui::mesh::Mesh;

#[derive(Clone)]
pub struct GuiIntegration {
    vertices: AllocatedBuffer<Vertex>,
    indices:  AllocatedBuffer<u32>,
    meshes:   Vec<Mesh>,
    textures: AHashMap<TextureId, AllocatedImage>,

    vertices_to_destroy: Vec<(usize, AllocatedBuffer<Vertex>)>,
    indices_to_destroy:  Vec<(usize, AllocatedBuffer<u32>)>,
    meshes_to_destroy:   Vec<(usize, Mesh)>,
    textures_to_destroy: Vec<(usize, AllocatedImage)>,
    frames_in_flight:    usize,
}

impl GuiIntegration {
    pub fn new(vk_ctx: VulkanContext, frames_in_flight: usize) -> Self {
        Self {
            vertices: AllocatedBuffer::new(
                vk_ctx.allocator.clone(),
                &vk::BufferCreateInfoBuilder::new()
                    .usage(vk::BufferUsageFlags::VERTEX_BUFFER)
                    .sharing_mode(vk::SharingMode::EXCLUSIVE)
                    .size(1024),
                vma::MemoryUsage::CpuToGpu,
            ),
            indices: AllocatedBuffer::new(
                vk_ctx.allocator,
                &vk::BufferCreateInfoBuilder::new()
                    .usage(vk::BufferUsageFlags::INDEX_BUFFER)
                    .sharing_mode(vk::SharingMode::EXCLUSIVE)
                    .size(1024),
                vma::MemoryUsage::CpuToGpu,
            ),
            meshes: vec![],
            textures: AHashMap::new(),
            vertices_to_destroy: vec![],
            indices_to_destroy: vec![],
            meshes_to_destroy: vec![],
            textures_to_destroy: vec![],
            frames_in_flight,
        }
    }

    pub fn destroy(&mut self, device: &DeviceLoader) {
        self.vertices.destroy();
        self.indices.destroy();
        for mesh in self.meshes.drain(..) {
            mesh.graphics_pipeline.destroy(device);
        }
        for (_, image) in self.textures.drain() {
            image.destroy(device);
        }
        for (_, vertex_buffer) in self.vertices_to_destroy.drain(..) {
            vertex_buffer.destroy();
        }
        for (_, index) in self.indices_to_destroy.drain(..) {
            index.destroy();
        }
        for (_, mesh) in self.meshes_to_destroy.drain(..) {
            mesh.graphics_pipeline.destroy(device);
        }
        for (_, image) in self.textures_to_destroy.drain(..) {
            image.destroy(device);
        }
    }

    pub fn update_gui_graphics(
        &mut self,
        vk_ctx: VulkanContext,
        surface_format: vk::SurfaceFormatKHR,
        textures_delta: &TexturesDelta,
        clipped_meshes: Vec<ClippedMesh>,
    ) {
        for (id, image_delta) in textures_delta.set.iter() {
            if self.textures.contains_key(id) {
                let image = &self.textures[id];
                let image_offset = match image_delta.pos {
                    Some([x, y]) => vk::Offset3D { x: x as i32, y: y as i32, z: 0 },
                    None => vk::Offset3D::default(),
                };
                match image_delta.image.borrow() {
                    ImageData::Alpha(a_image) => {
                        let texture_data = a_image.srgba_pixels(1.0).collect_vec();
                        image.set_data(vk_ctx.clone(), image_offset, &texture_data)
                    }
                    ImageData::Color(c_image) => image.set_data(vk_ctx.clone(), image_offset, &c_image.pixels),
                }
            } else {
                assert!(image_delta.pos.is_none());
                let image = match image_delta.image.borrow() {
                    ImageData::Alpha(a_image) => {
                        let texture_data = a_image.srgba_pixels(1.0).collect_vec();
                        AllocatedImage::texture_with_data(
                            vk_ctx.clone(),
                            vk::Format::R8G8B8A8_UNORM,
                            vk::Extent3D { width: a_image.size[0] as u32, height: a_image.size[1] as u32, depth: 1 },
                            vk::ImageViewType::_2D,
                            1,
                            1,
                            &texture_data,
                        )
                    }
                    ImageData::Color(c_image) => AllocatedImage::texture_with_data(
                        vk_ctx.clone(),
                        vk::Format::R8G8B8A8_SRGB,
                        vk::Extent3D { width: c_image.size[0] as u32, height: c_image.size[1] as u32, depth: 1 },
                        vk::ImageViewType::_2D,
                        1,
                        1,
                        &c_image.pixels,
                    ),
                };
                self.textures.insert(*id, image);
            }
        }

        for id in textures_delta.free.iter() {
            if let Some(image) = self.textures.remove(id) {
                self.textures_to_destroy.push((self.frames_in_flight, image));
            }
        }

        self.meshes_to_destroy.extend(self.meshes.drain(..).map(|m| (self.frames_in_flight, m)));

        let mut vertices = vec![];
        let mut indices = vec![];
        for ClippedMesh(clip, mesh) in clipped_meshes.into_iter() {
            let vertex_offset = vertices.len() as i32;
            let first_index = indices.len() as u32;
            let index_count = mesh.indices.len() as u32;
            let texture_id = mesh.texture_id;
            let scissor = vk::Rect2D {
                offset: vk::Offset2D { x: clip.min.x as i32, y: clip.min.y as i32 },
                extent: vk::Extent2D {
                    width:  (clip.max.x - clip.min.x) as u32,
                    height: (clip.max.y - clip.min.y) as u32,
                },
            };

            vertices.extend(mesh.vertices.into_iter());
            indices.extend(mesh.indices.into_iter());
            self.meshes.push(Mesh::new(
                vk_ctx.clone(),
                vertex_offset,
                first_index,
                index_count,
                surface_format,
                self.textures[&texture_id].view,
                scissor,
            ));
        }

        let vertex_buf_size = (vertices.len() * std::mem::size_of::<Vertex>()) as vk::DeviceSize;
        let index_buf_size = (indices.len() * std::mem::size_of::<u32>()) as vk::DeviceSize;

        if vertex_buf_size > self.vertices.size {
            let new_vertices = AllocatedBuffer::new(
                vk_ctx.allocator.clone(),
                &vk::BufferCreateInfoBuilder::new()
                    .usage(vk::BufferUsageFlags::VERTEX_BUFFER)
                    .sharing_mode(vk::SharingMode::EXCLUSIVE)
                    .size(vertex_buf_size),
                vma::MemoryUsage::CpuToGpu,
            );
            self.vertices_to_destroy.push((self.frames_in_flight, std::mem::replace(&mut self.vertices, new_vertices)));
        }
        if index_buf_size > self.indices.size {
            let new_indices = AllocatedBuffer::new(
                vk_ctx.allocator,
                &vk::BufferCreateInfoBuilder::new()
                    .usage(vk::BufferUsageFlags::INDEX_BUFFER)
                    .sharing_mode(vk::SharingMode::EXCLUSIVE)
                    .size(index_buf_size),
                vma::MemoryUsage::CpuToGpu,
            );
            self.indices_to_destroy.push((self.frames_in_flight, std::mem::replace(&mut self.indices, new_indices)));
        }

        self.vertices.set_data(&vertices);
        self.indices.set_data(&indices);
    }

    fn spin_cleanups(&mut self, device: &DeviceLoader) {
        fn spin<T, D>(deletables: &mut Vec<(usize, T)>, destroy_fn: D)
        where
            D: Fn(&T),
        {
            *deletables = std::mem::take(deletables)
                .into_iter()
                .filter_map(|(lifetime, deletable)| {
                    if lifetime == 0 {
                        destroy_fn(&deletable);
                        None
                    } else {
                        Some((lifetime - 1, deletable))
                    }
                })
                .collect();
        }

        spin(&mut self.vertices_to_destroy, AllocatedBuffer::destroy);
        spin(&mut self.indices_to_destroy, AllocatedBuffer::destroy);
        spin(&mut self.meshes_to_destroy, |mesh| mesh.graphics_pipeline.destroy(device));
        spin(&mut self.textures_to_destroy, |texture| texture.destroy(device));
    }

    pub fn render(&mut self, ctx: RenderContext) {
        self.spin_cleanups(ctx.device);

        let RenderContext { device, command_buffer, screen_extent } = ctx;
        let push_constants =
            GuiPushConstants { screen_size: glm::vec2(screen_extent.width as f32, screen_extent.height as f32) };

        unsafe {
            device.cmd_bind_vertex_buffers(command_buffer, 0, &[self.vertices.buffer], &[0]);
            device.cmd_bind_index_buffer(command_buffer, self.indices.buffer, 0, vk::IndexType::UINT32);
        }

        for mesh in self.meshes.iter() {
            unsafe {
                device.cmd_push_constants(
                    command_buffer,
                    mesh.graphics_pipeline.layout,
                    vk::ShaderStageFlags::VERTEX,
                    0,
                    std::mem::size_of::<GuiPushConstants>() as u32,
                    &push_constants as *const GuiPushConstants as *const c_void,
                );
                device.cmd_set_scissor(command_buffer, 0, &[mesh.scissor.into_builder()]);
                device.cmd_bind_pipeline(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    mesh.graphics_pipeline.pipeline,
                );
                device.cmd_bind_descriptor_sets(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    mesh.graphics_pipeline.layout,
                    0,
                    &mesh.graphics_pipeline.descriptor_sets,
                    &[],
                );
                device.cmd_draw_indexed(command_buffer, mesh.index_count, 1, mesh.first_index, mesh.vertex_offset, 0);
            }
        }
    }
}
