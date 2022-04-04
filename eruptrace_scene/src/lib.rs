#![feature(iter_partition_in_place)]
#![feature(total_cmp)]

pub mod bih;
pub mod camera;
pub mod json;
pub mod materials;
pub mod mesh;

pub use camera::Camera;

use crate::{bih::Bih, json::to_vec3, materials::*, mesh::*};
use itertools::Itertools;
use nalgebra_glm as glm;
use serde_json as js;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone)]
pub struct Scene {
    pub meshes: Vec<Mesh>,
    pub triangles: Vec<Triangle>,
    pub materials: Vec<Material>,
    pub texture_paths: Vec<PathBuf>,
    pub normal_map_paths: Vec<PathBuf>,
    pub bih: Bih,
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
                triangles.extend(m_triangles
                    .into_iter()
                    .map(|t| Triangle {
                        positions: t.positions.map(|p| {
                            (mesh.transform * glm::vec4(p.x, p.y, p.z, 1.0)).xyz()
                        }),
                        normals: t.normals.map(|n| {
                            (glm::transpose(&glm::inverse(&mesh.transform)) * glm::vec4(n.x, n.y, n.z, 1.0)).xyz()
                        }),
                        texcoords: t.texcoords,
                        material_index: t.material_index,
                    })
                );
                meshes.push(mesh);
            }

            let bih = Bih::new(&mut triangles);

            Self {
                meshes,
                triangles,
                materials,
                texture_paths,
                normal_map_paths,
                bih,
            }
        };

        Ok((camera, scene))
    }
}
