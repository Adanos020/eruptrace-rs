pub mod camera;
pub mod json;
pub mod materials;
pub mod shapes;

use crate::json::to_vec3;
use crate::{camera::Camera, materials::*, shapes::*};
use serde_json as js;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub struct Scene {
    pub spheres: Vec<Sphere>,
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
    pub texture_paths: Vec<PathBuf>,
}

impl Scene {
    pub fn load<P: AsRef<Path>>(scene_path: P) -> anyhow::Result<(Camera, Self)> {
        let camera = {
            let mut cam_path = PathBuf::new();
            cam_path.push(&scene_path);
            cam_path.push("camera.json");
            let file_contents = fs::read_to_string(cam_path)?;
            let cam_json: js::Value = js::from_str(&file_contents)?;

            let position = to_vec3(&cam_json["position"]);
            let look_at = to_vec3(&cam_json["look_at"]);
            let up = to_vec3(&cam_json["up"]);
            let vertical_fov = cam_json["fov"].as_f64().unwrap_or(90.0) as f32;
            let samples = cam_json["samples"].as_u64().unwrap_or(1) as u32;
            let max_reflections = cam_json["max_reflections"].as_u64().unwrap_or(1) as u32;

            Camera {
                position,
                look_at,
                up,
                img_size: [0, 0],
                vertical_fov,
                samples,
                max_reflections,
            }
        };

        let scene = {
            let mut desc_path = PathBuf::new();
            desc_path.push(&scene_path);
            desc_path.push("scene.json");
            let file_contents = fs::read_to_string(desc_path)?;
            let scene_json: js::Value = js::from_str(&file_contents)?;

            let (texture_names, texture_paths) = {
                let obj = scene_json["textures"].as_object().unwrap();
                let mut names: Vec<String> = obj.keys().map(|n| n.to_owned()).collect();
                let mut paths: Vec<PathBuf> = obj
                    .values()
                    .filter(|p| p.is_string())
                    .map(|p| {
                        let mut tex_path = PathBuf::new();
                        tex_path.push(&scene_path);
                        tex_path.push("textures");
                        tex_path.push(p.as_str().unwrap());
                        tex_path
                    })
                    .collect();
                if let Some(sky_idx) = names.iter().position(|n| n == "sky") {
                    unsafe {
                        std::ptr::swap(&mut paths[0], &mut paths[sky_idx]);
                        std::ptr::swap(&mut names[0], &mut names[sky_idx]);
                    }
                } else {
                    eprintln!("Missing 'sky' texture.");
                }
                (names, paths)
            };

            let (material_names, materials) = {
                let obj = scene_json["materials"].as_object().unwrap();
                let names: Vec<String> = obj.keys().map(|n| n.to_owned()).collect();
                let materials: Vec<Material> = obj
                    .values()
                    .filter(|m| m.is_object())
                    .map(|m| Material::from_json(m, &texture_names).unwrap())
                    .collect();
                (names, materials)
            };

            let spheres = scene_json["spheres"]
                .as_array()
                .map_or(&vec![], |v| v)
                .iter()
                .filter(|s| s.is_object())
                .map(|s| Sphere::from_json(s, &material_names).unwrap())
                .collect();

            let meshes = scene_json["meshes"]
                .as_array()
                .map_or(&vec![], |v| v)
                .iter()
                .filter(|m| m.is_object())
                .map(|m| Mesh::from_json(m, &material_names).unwrap())
                .collect();

            Self {
                spheres,
                meshes,
                materials,
                texture_paths,
            }
        };

        Ok((camera, scene))
    }
}
