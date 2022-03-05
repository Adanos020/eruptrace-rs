pub mod camera;
pub mod json;
pub mod materials;
pub mod shapes;

use crate::{materials::*, shapes::*};
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
    pub fn load<P: AsRef<Path>>(scene_path: P) -> anyhow::Result<Self> {
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

        Ok(Self {
            spheres,
            meshes,
            materials,
            texture_paths,
        })
    }
}
