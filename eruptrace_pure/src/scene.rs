#![allow(clippy::no_effect)]

use erupt::{vk, DeviceLoader};
use eruptrace_scene::{
    materials::Material,
    shapes::{Mesh, Sphere},
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
    pub shapes_buffer: AllocatedBuffer<f32>,
    pub mesh_metas_buffer: AllocatedBuffer<MeshMetaStd140>,
    pub mesh_data_buffer: AllocatedBuffer<f32>,
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
pub struct MeshMetaStd140 {
    pub material_index: uint,
    pub positions_start: uint,
    pub normals_start: uint,
    pub texcoords_start: uint,
    pub indices_start: uint,
    pub mesh_end: uint,
}

impl SceneBuffers {
    pub fn create_buffers(
        allocator: Arc<RwLock<vma::Allocator>>,
        vk_ctx: VulkanContext,
        scene: Scene,
    ) -> vma::Result<Self> {
        let n_textures = scene.texture_paths.len();
        let n_normal_maps = scene.normal_map_paths.len();
        let textures = get_image_data(scene.texture_paths);
        let normal_maps = get_image_data(scene.normal_map_paths);
        let materials = get_material_data(scene.materials);
        let shapes = get_shapes_data(scene.spheres);
        let (mesh_metas, mesh_data) = get_mesh_data(scene.meshes);

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
            shapes_buffer: AllocatedBuffer::with_data(
                allocator.clone(),
                &buffer_info,
                buffer_allocation_info.clone(),
                &shapes,
            )?,
            mesh_metas_buffer: AllocatedBuffer::with_data(
                allocator.clone(),
                &buffer_info,
                buffer_allocation_info.clone(),
                &mesh_metas,
            )?,
            mesh_data_buffer: AllocatedBuffer::with_data(
                allocator,
                &buffer_info,
                buffer_allocation_info,
                &mesh_data,
            )?,
        })
    }

    pub fn destroy(&self, device: &DeviceLoader) {
        self.textures_image.destroy(device);
        self.normal_maps_image.destroy(device);
        self.materials_buffer.destroy();
        self.shapes_buffer.destroy();
        self.mesh_metas_buffer.destroy();
        self.mesh_data_buffer.destroy();
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

fn get_shapes_data(spheres: Vec<Sphere>) -> Vec<f32> {
    let mut data = Vec::with_capacity(1 + (spheres.len() * std::mem::size_of::<Sphere>()));

    data.push(spheres.len() as f32);
    for sphere in spheres.into_iter() {
        data.push(sphere.position.x);
        data.push(sphere.position.y);
        data.push(sphere.position.z);
        data.push(sphere.radius as f32);
        data.push(sphere.material_index as f32);
    }

    data
}

fn get_mesh_data(meshes: Vec<Mesh>) -> (Vec<MeshMetaStd140>, Vec<f32>) {
    let mut metas = Vec::with_capacity(meshes.len());
    let mut data = Vec::with_capacity(1 + meshes.iter().map(|m| m.size_in_f32s()).sum::<usize>());
    let mut curr_mesh_start = 1u32;

    // Number of meshes
    data.push(meshes.len() as f32);
    for mesh in meshes.into_iter() {
        let positions_size = mesh.positions.len() * 3;
        let normals_size = mesh.normals.len() * 3;
        let texcoords_size = mesh.texcoords.len() * 2;
        let indices_size = mesh.indices.len();

        let material_index = uint(mesh.material_index);
        let positions_start = uint(curr_mesh_start);
        let normals_start = uint(positions_start.0 + positions_size as u32);
        let texcoords_start = uint(normals_start.0 + normals_size as u32);
        let indices_start = uint(texcoords_start.0 + texcoords_size as u32);
        let mesh_end = uint(indices_start.0 + indices_size as u32);

        metas.push(MeshMetaStd140 {
            material_index,
            positions_start,
            normals_start,
            texcoords_start,
            indices_start,
            mesh_end,
        });

        for position in mesh.positions.into_iter() {
            data.push(position.x);
            data.push(position.y);
            data.push(position.z);
        }
        for normal in mesh.normals.into_iter() {
            data.push(normal.x);
            data.push(normal.y);
            data.push(normal.z);
        }
        for texcoord in mesh.texcoords.into_iter() {
            data.push(texcoord.x);
            data.push(texcoord.y);
        }
        for index in mesh.indices.into_iter() {
            data.push(index as f32);
        }

        curr_mesh_start = mesh_end.0;
    }

    (metas, data)
}
