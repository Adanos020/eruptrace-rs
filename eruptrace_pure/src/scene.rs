#![allow(clippy::no_effect)]

use erupt::{vk, DeviceLoader};
use eruptrace_scene::{
    bih::{Bih, BihNodeData},
    materials::Material,
    mesh::Triangle,
    Scene,
};
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
    pub bih_buffer: AllocatedBuffer<BihNodeStd140>,
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
#[derive(Copy, Clone, Debug)]
pub struct BihNodeStd140 {
    pub node_type: uint,
    pub child_left: uint,
    pub child_right: uint,
    pub clip_left: float,
    pub clip_right: float,
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
    ) -> Self {
        let n_textures = scene.texture_paths.len();
        let n_normal_maps = scene.normal_map_paths.len();
        let n_triangles = scene.meshes.len() as u32;
        let textures = get_image_data(scene.texture_paths);
        let normal_maps = get_image_data(scene.normal_map_paths);
        let materials = get_material_data(scene.materials);
        let triangles = get_triangle_data(scene.triangles);
        let bih = get_bih_data(scene.bih);

        let image_extent = vk::Extent3D {
            width: 1024,
            height: 1024,
            depth: 1,
        };

        let buffer_info = vk::BufferCreateInfoBuilder::new()
            .usage(vk::BufferUsageFlags::STORAGE_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        SceneBuffers {
            textures_image: AllocatedImage::texture(
                vk_ctx.clone(),
                allocator.clone(),
                image_extent,
                vk::ImageViewType::_2D_ARRAY,
                1,
                n_textures as u32,
                &textures,
            ),
            normal_maps_image: AllocatedImage::texture(
                vk_ctx,
                allocator.clone(),
                image_extent,
                vk::ImageViewType::_2D_ARRAY,
                1,
                n_normal_maps as u32,
                &normal_maps,
            ),
            materials_buffer: AllocatedBuffer::with_data(
                allocator.clone(),
                &buffer_info,
                vma::MemoryUsage::CpuToGpu,
                &materials,
            ),
            triangles_buffer: AllocatedBuffer::with_data(
                allocator.clone(),
                &buffer_info,
                vma::MemoryUsage::CpuToGpu,
                &triangles,
            ),
            bih_buffer: AllocatedBuffer::with_data(
                allocator,
                &buffer_info,
                vma::MemoryUsage::CpuToGpu,
                &bih,
            ),
            n_triangles,
        }
    }

    pub fn destroy(&self, device: &DeviceLoader) {
        self.textures_image.destroy(device);
        self.normal_maps_image.destroy(device);
        self.materials_buffer.destroy();
        self.triangles_buffer.destroy();
        self.bih_buffer.destroy();
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
                eruptrace_vk::std140::vec3(&t.positions[0]),
                eruptrace_vk::std140::vec3(&t.positions[1]),
                eruptrace_vk::std140::vec3(&t.positions[2]),
            ],
            normals: array![
                eruptrace_vk::std140::vec3(&t.normals[0]),
                eruptrace_vk::std140::vec3(&t.normals[1]),
                eruptrace_vk::std140::vec3(&t.normals[2]),
            ],
            texcoords: array![
                eruptrace_vk::std140::vec2(&t.texcoords[0]),
                eruptrace_vk::std140::vec2(&t.texcoords[1]),
                eruptrace_vk::std140::vec2(&t.texcoords[2]),
            ],
            material_index: uint(t.material_index),
        })
        .collect()
}

fn get_bih_data(bih: Bih) -> Vec<BihNodeStd140> {
    bih.0
        .into_iter()
        .map(|node| match node.data {
            BihNodeData::Branch {
                clip_left,
                clip_right,
                child_left,
                child_right,
            } => BihNodeStd140 {
                node_type: uint(node.ty as u32),
                child_left: uint(child_left as u32),
                child_right: uint(child_right as u32),
                clip_left: float(clip_left),
                clip_right: float(clip_right),
            },
            BihNodeData::Leaf {
                triangle_index,
                count,
            } => BihNodeStd140 {
                node_type: uint(node.ty as u32),
                child_left: uint(triangle_index as u32),
                child_right: uint(count as u32),
                clip_left: float(0.0),
                clip_right: float(0.0),
            },
        })
        .collect()
}
