#![allow(clippy::no_effect)]

use erupt::{vk, DeviceLoader};
use eruptrace_scene::mesh::Triangle;
use eruptrace_scene::{materials::Material, Scene};
use eruptrace_vk::{AllocatedBuffer, AllocatedImage, VulkanContext};
use image::EncodableLayout;
use std::{
    path::PathBuf,
    sync::{Arc, RwLock},
};
use std140::*;
use vk_mem_erupt as vma;

#[derive(Clone)]
pub struct SceneBuffers {
    pub textures_image: AllocatedImage,
    pub normal_maps_image: AllocatedImage,
    pub materials_buffer: AllocatedBuffer<MaterialStd140>,
    pub triangles_buffer: AllocatedBuffer<TriangleStd140>,
    pub n_triangles: u32,
}

#[repr_std140]
#[derive(Copy, Clone, Debug)]
pub struct MaterialStd140 {
    pub material_type: uint,
    pub texture_index: uint,
    pub normal_map_index: uint,
    pub parameter: float,
}

#[repr_std140]
#[derive(Clone, Debug)]
pub struct TriangleStd140 {
    pub positions: array<vec3, 3>,
    pub normals: array<vec3, 3>,
    pub texcoords: array<vec2, 3>,
    pub material_index: uint,
}

impl SceneBuffers {
    pub fn create_buffers(
        allocator: Arc<RwLock<vma::Allocator>>,
        vk_ctx: VulkanContext,
        scene: Scene,
    ) -> vma::Result<Self> {
        let n_textures = scene.texture_paths.len();
        let n_normal_maps = scene.normal_map_paths.len();
        let n_triangles = scene.triangles.len() as u32;
        let textures = get_image_data(scene.texture_paths);
        let normal_maps = get_image_data(scene.normal_map_paths);
        let materials = get_material_data(scene.materials);
        let triangles = get_triangle_data(scene.triangles);

        let image_extent = vk::Extent3D {
            width: 1024,
            height: 1024,
            depth: 1,
        };

        let buffer_info = vk::BufferCreateInfoBuilder::new()
            .usage(vk::BufferUsageFlags::STORAGE_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        let buffer_allocation_info = vma::AllocationCreateInfo {
            usage: vma::MemoryUsage::CpuToGpu,
            flags: vma::AllocationCreateFlags::DEDICATED_MEMORY
                | vma::AllocationCreateFlags::MAPPED,
            ..Default::default()
        };

        Ok(SceneBuffers {
            textures_image: AllocatedImage::with_data(
                vk_ctx.clone(),
                allocator.clone(),
                vk::Format::R8G8B8A8_UNORM,
                image_extent,
                vk::ImageViewType::_2D_ARRAY,
                1,
                n_textures as u32,
                &textures,
            )?,
            normal_maps_image: AllocatedImage::with_data(
                vk_ctx,
                allocator.clone(),
                vk::Format::R8G8B8A8_UNORM,
                image_extent,
                vk::ImageViewType::_2D_ARRAY,
                1,
                n_normal_maps as u32,
                &normal_maps,
            )?,
            materials_buffer: AllocatedBuffer::with_data(
                allocator.clone(),
                &buffer_info,
                buffer_allocation_info.clone(),
                &materials,
            )?,
            triangles_buffer: AllocatedBuffer::with_data(
                allocator,
                &buffer_info,
                buffer_allocation_info,
                &triangles,
            )?,
            n_triangles,
        })
    }

    pub fn destroy(&self, device: &DeviceLoader) {
        self.textures_image.destroy(device);
        self.normal_maps_image.destroy(device);
        self.materials_buffer.destroy();
        self.triangles_buffer.destroy();
    }
}

fn get_image_data(image_paths: Vec<PathBuf>) -> Vec<u8> {
    image_paths
        .into_iter()
        .map(|path| image::open(path).unwrap().into_rgba8())
        .flat_map(|texture| Vec::from(texture.as_bytes()))
        .collect()
}

fn get_material_data(materials: Vec<Material>) -> Vec<MaterialStd140> {
    let to_std140 = |mat: Material| MaterialStd140 {
        material_type: uint(mat.material_type as u32),
        texture_index: uint(mat.texture_index),
        normal_map_index: uint(mat.normal_map_index),
        parameter: float(mat.parameter),
    };
    materials.into_iter().map(to_std140).collect()
}

fn get_triangle_data(triangles: Vec<Triangle>) -> Vec<TriangleStd140> {
    triangles
        .into_iter()
        .map(|t| TriangleStd140 {
            positions: array![
                vec3(t.positions[0][0], t.positions[0][1], t.positions[0][2]),
                vec3(t.positions[1][0], t.positions[1][1], t.positions[1][2]),
                vec3(t.positions[2][0], t.positions[2][1], t.positions[2][2]),
            ],
            normals: array![
                vec3(t.normals[0][0], t.normals[0][1], t.normals[0][2]),
                vec3(t.normals[1][0], t.normals[1][1], t.normals[1][2]),
                vec3(t.normals[2][0], t.normals[2][1], t.normals[2][2]),
            ],
            texcoords: array![
                vec2(t.texcoords[0][0], t.texcoords[0][1]),
                vec2(t.texcoords[1][0], t.texcoords[1][1]),
                vec2(t.texcoords[2][0], t.texcoords[2][1]),
            ],
            material_index: uint(t.material_index),
        })
        .collect()
}
