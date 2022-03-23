use crate::json::{to_vec2, to_vec3};
use nalgebra_glm as glm;
use serde_json as js;

#[derive(Clone, Debug)]
pub struct Mesh {
    pub positions: Vec<glm::TVec3<f32>>,
    pub normals: Vec<glm::TVec3<f32>>,
    pub texcoords: Vec<glm::TVec2<f32>>,
    pub indices: Vec<u32>,
    pub material_index: u32,
}

impl Mesh {
    pub fn from_json(object: &js::Value, material_names: &[String]) -> anyhow::Result<Self> {
        let positions: Vec<glm::TVec3<f32>> = object["positions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| p.is_array())
            .map(to_vec3)
            .collect();

        let normals: Vec<glm::TVec3<f32>> = object["normals"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| p.is_array())
            .map(to_vec3)
            .collect();

        let texcoords: Vec<glm::TVec2<f32>> = object["texcoords"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| p.is_array())
            .map(to_vec2)
            .collect();

        let indices: Vec<u32> = object["indices"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| p.is_u64())
            .map(|p| p.as_u64().unwrap_or(0) as u32)
            .collect();

        let material_index = material_names
            .iter()
            .position(|n| object["material"] == *n)
            .unwrap_or_default() as u32;

        Ok(Self {
            positions,
            normals,
            texcoords,
            indices,
            material_index,
        })
    }

    pub fn size_in_f32s(&self) -> usize {
        self.positions.len() * 3
            + self.normals.len() * 3
            + self.texcoords.len() * 2
            + self.indices.len()
            + 1
    }
}
