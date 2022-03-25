use itertools::Itertools;
use crate::json::{to_vec2, to_vec3};
use nalgebra_glm as glm;
use serde_json as js;
use crate::bih::BoundingBox;

#[derive(Clone, Debug)]
pub struct Triangle {
    pub vertices: [glm::Vec3; 3],
}

#[derive(Clone, Debug)]
pub struct Mesh {
    pub positions: Vec<glm::Vec3>,
    pub normals: Vec<glm::Vec3>,
    pub texcoords: Vec<glm::Vec2>,
    pub indices: Vec<u32>,
    pub material_index: u32,
}

impl Mesh {
    pub fn from_json(object: &js::Value, material_names: &[String]) -> anyhow::Result<Self> {
        let positions: Vec<glm::Vec3> = object["positions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| p.is_array())
            .map(to_vec3)
            .collect();

        let normals: Vec<glm::Vec3> = object["normals"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| p.is_array())
            .map(to_vec3)
            .collect();

        let texcoords: Vec<glm::Vec2> = object["texcoords"]
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

    pub fn triangles(&self) -> Vec<Triangle> {
        self.indices
            .iter()
            .tuple_windows::<(_, _, _)>()
            .map(|(&a, &b, &c)| Triangle {
                 vertices: [
                     self.positions[a as usize],
                     self.positions[b as usize],
                     self.positions[c as usize],
                 ]
            })
            .collect()
    }
}

impl Triangle {
    pub fn bounds(&self) -> BoundingBox {
        let [a, b, c] = self.vertices;
        BoundingBox {
            min: glm::vec3(
                a[0].min(b[0]).min(c[0]),
                a[1].min(b[1]).min(c[1]),
                a[2].min(b[2]).min(c[2]),
            ),
            max: glm::vec3(
                a[0].max(b[0]).max(c[0]),
                a[1].max(b[1]).max(c[1]),
                a[2].max(b[2]).max(c[2]),
            ),
        }
    }
}
