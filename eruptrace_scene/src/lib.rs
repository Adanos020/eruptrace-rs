pub mod camera;
pub mod json;
pub mod materials;
pub mod shapes;

use crate::{camera::Camera, json::to_vec3, materials::*, shapes::*};
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
    pub normal_map_paths: Vec<PathBuf>,
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
                normal_map_paths,
            }
        };

        Ok((camera, scene))
    }
}
