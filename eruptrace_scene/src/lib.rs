#![feature(iter_partition_in_place)]
#![feature(total_cmp)]
#![allow(clippy::no_effect)]

pub mod bih;
pub mod camera;
pub mod json;
pub mod materials;
pub mod mesh;

use std::{
    fs,
    path::{Path, PathBuf},
};

pub use bih::*;
pub use camera::*;
use erupt::{vk, DeviceLoader};
use eruptrace_vk::{AllocatedBuffer, AllocatedImage, VulkanContext};
use image::EncodableLayout;
use itertools::Itertools;
pub use materials::*;
pub use mesh::*;
use nalgebra_glm as glm;
use serde_json as js;
use vk_mem_erupt as vma;

use crate::json::to_vec3;

#[derive(Clone)]
pub struct Scene {
    pub meshes:           Vec<Mesh>,
    pub triangles:        Vec<Triangle>,
    pub materials:        Vec<Material>,
    pub texture_paths:    Vec<PathBuf>,
    pub normal_map_paths: Vec<PathBuf>,
    pub bih:              Bih,
}

#[derive(Clone)]
pub struct SceneBuffers {
    pub textures_image:    AllocatedImage,
    pub normal_maps_image: AllocatedImage,
    pub materials_buffer:  AllocatedBuffer<MaterialUniform>,
    pub triangles_buffer:  AllocatedBuffer<TriangleUniform>,
    pub bih_buffer:        AllocatedBuffer<BihNodeUniform>,
    pub n_triangles:       u32,
}

impl Scene {
    pub fn load<P: AsRef<Path>>(scene_path: P) -> anyhow::Result<(Camera, Self)> {
        let camera = {
            let mut cam_path = PathBuf::new();
            cam_path.push(&scene_path);
            cam_path.push("camera.json");
            let file_contents = fs::read_to_string(cam_path)?;
            let cam_json = js::from_str(&file_contents)?;
            Camera::from_json(cam_json)?
        };

        let scene = {
            let mut desc_path = PathBuf::new();
            desc_path.push(&scene_path);
            desc_path.push("scene.json");
            let file_contents = fs::read_to_string(desc_path)?;
            let scene_json: js::Value = js::from_str(&file_contents)?;

            let get_paths = |res, first_name| {
                let obj = scene_json[res].as_object().unwrap();
                let mut names: Vec<String> = obj.keys().map(|n| n.to_owned()).collect();
                let mut paths: Vec<PathBuf> = obj
                    .values()
                    .filter(|p| p.is_string())
                    .map(|p| {
                        let mut tex_path = PathBuf::new();
                        tex_path.push(&scene_path);
                        tex_path.push(res);
                        tex_path.push(p.as_str().unwrap());
                        tex_path
                    })
                    .collect();
                if let Some(first_idx) = names.iter().position(|n| n == first_name) {
                    unsafe {
                        std::ptr::swap(&mut paths[0], &mut paths[first_idx]);
                        std::ptr::swap(&mut names[0], &mut names[first_idx]);
                    }
                } else {
                    eprintln!("Missing 'sky' texture.");
                }
                (names, paths)
            };

            let (texture_names, texture_paths) = get_paths("textures", "sky");
            let (normal_map_names, normal_map_paths) = get_paths("normal_maps", "default");

            let (material_names, materials) = {
                let obj = scene_json["materials"].as_object().unwrap();
                let names: Vec<String> = obj.keys().map(|n| n.to_owned()).collect();
                let materials: Vec<Material> = obj
                    .values()
                    .filter(|m| m.is_object())
                    .map(|m| Material::from_json(m, &texture_names, &normal_map_names).unwrap())
                    .collect();
                (names, materials)
            };

            let meshes_and_triangles = scene_json["meshes"]
                .as_array()
                .map_or(&vec![], |v| v)
                .iter()
                .filter(|m| m.is_object())
                .map(|m| Mesh::from_json(m, &material_names))
                .filter_map(|m| match m {
                    Ok(m) => {
                        let t = m.triangles();
                        Some((m, t))
                    }
                    Err(e) => {
                        eprintln!("{}", e);
                        None
                    }
                })
                .collect_vec();
            let mut meshes = Vec::with_capacity(meshes_and_triangles.len());
            let mut triangles = Vec::with_capacity(meshes_and_triangles.len());
            for (mesh, m_triangles) in meshes_and_triangles.into_iter() {
                triangles.extend(m_triangles.into_iter().map(|t| Triangle {
                    positions:      t.positions.map(|p| (mesh.transform * glm::vec4(p.x, p.y, p.z, 1.0)).xyz()),
                    normals:        t.normals.map(|n| {
                        let normal_transform = glm::transpose(&glm::inverse(&mesh.transform));
                        (normal_transform * glm::vec4(n.x, n.y, n.z, 1.0)).xyz()
                    }),
                    texcoords:      t.texcoords,
                    material_index: t.material_index,
                }));
                meshes.push(mesh);
            }

            let bih = Bih::new(&mut triangles);

            Self { meshes, triangles, materials, texture_paths, normal_map_paths, bih }
        };

        Ok((camera, scene))
    }

    pub fn create_buffers(self, vk_ctx: VulkanContext) -> SceneBuffers {
        let n_textures = self.texture_paths.len();
        let n_normal_maps = self.normal_map_paths.len();
        let n_triangles = self.meshes.len() as u32;
        let textures = self
            .texture_paths
            .into_iter()
            .map(|path| image::open(path).unwrap().into_rgba8())
            .flat_map(|texture| Vec::from(texture.as_bytes()))
            .collect_vec();
        let normal_maps = self
            .normal_map_paths
            .into_iter()
            .map(|path| image::open(path).unwrap().into_rgba8())
            .flat_map(|texture| Vec::from(texture.as_bytes()))
            .collect_vec();
        let materials = self.materials.into_iter().map(Material::into_uniform).collect_vec();
        let triangles = self.triangles.into_iter().map(Triangle::into_uniform).collect_vec();
        let bih = self.bih.0.into_iter().map(BihNode::into_uniform).collect_vec();

        let image_extent = vk::Extent3D { width: 1024, height: 1024, depth: 1 };

        let buffer_info = vk::BufferCreateInfoBuilder::new()
            .usage(vk::BufferUsageFlags::STORAGE_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        SceneBuffers {
            textures_image: AllocatedImage::texture(
                vk_ctx.clone(),
                image_extent,
                vk::ImageViewType::_2D_ARRAY,
                1,
                n_textures as u32,
                &textures,
            ),
            normal_maps_image: AllocatedImage::texture(
                vk_ctx.clone(),
                image_extent,
                vk::ImageViewType::_2D_ARRAY,
                1,
                n_normal_maps as u32,
                &normal_maps,
            ),
            materials_buffer: AllocatedBuffer::with_data(
                vk_ctx.allocator.clone(),
                &buffer_info,
                vma::MemoryUsage::CpuToGpu,
                &materials,
            ),
            triangles_buffer: AllocatedBuffer::with_data(
                vk_ctx.allocator.clone(),
                &buffer_info,
                vma::MemoryUsage::CpuToGpu,
                &triangles,
            ),
            bih_buffer: AllocatedBuffer::with_data(vk_ctx.allocator, &buffer_info, vma::MemoryUsage::CpuToGpu, &bih),
            n_triangles,
        }
    }
}

impl SceneBuffers {
    pub fn destroy(&self, device: &DeviceLoader) {
        self.textures_image.destroy(device);
        self.normal_maps_image.destroy(device);
        self.materials_buffer.destroy();
        self.triangles_buffer.destroy();
        self.bih_buffer.destroy();
    }
}
