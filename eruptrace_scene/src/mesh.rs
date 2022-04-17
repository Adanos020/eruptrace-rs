use itertools::Itertools;
use nalgebra_glm as glm;
use serde_json as js;
use std140::repr_std140;

use crate::{
    bih::BoundingBox,
    json::{to_vec2, to_vec3},
};

#[derive(Clone, Debug)]
pub struct Triangle {
    pub positions:      [glm::Vec3; 3],
    pub normals:        [glm::Vec3; 3],
    pub texcoords:      [glm::Vec2; 3],
    pub material_index: u32,
}

#[repr_std140]
#[derive(Clone, Debug)]
pub struct TriangleUniform {
    pub positions:      std140::array<std140::vec3, 3>,
    pub normals:        std140::array<std140::vec3, 3>,
    pub texcoords:      std140::array<std140::vec2, 3>,
    pub material_index: std140::uint,
}

#[derive(Clone, Debug)]
pub struct Mesh {
    pub positions:      Vec<glm::Vec3>,
    pub normals:        Vec<glm::Vec3>,
    pub texcoords:      Vec<glm::Vec2>,
    pub indices:        Vec<u32>,
    pub transform:      glm::Mat4x4,
    pub material_index: u32,
}

impl Mesh {
    pub fn from_json(object: &js::Value, material_names: &[String]) -> anyhow::Result<Self> {
        let positions = object["positions"].as_array().unwrap().iter().filter_map(to_vec3).collect_vec();
        let normals = object["normals"].as_array().unwrap().iter().filter_map(to_vec3).collect_vec();
        let texcoords = object["texcoords"].as_array().unwrap().iter().filter_map(to_vec2).collect_vec();

        let indices = object["indices"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(serde_json::Value::as_u64)
            .map(|i| i as u32)
            .collect_vec();

        let transform: glm::Mat4x4 = match object["transform"].as_object() {
            Some(object) => {
                let get_transform = |name, func: &dyn Fn(&glm::Vec3) -> glm::Mat4x4| {
                    object.get(name).and_then(to_vec3).map(|vector| func(&vector)).unwrap_or_else(glm::identity)
                };
                let translation = get_transform("position", &glm::translation::<f32>);
                let rotation = get_transform("rotation", &|vec| {
                    let rot_x = glm::rotation(vec.x.to_radians(), &glm::vec3(1.0, 0.0, 0.0));
                    let rot_y = glm::rotation(vec.y.to_radians(), &glm::vec3(0.0, 1.0, 0.0));
                    let rot_z = glm::rotation(vec.z.to_radians(), &glm::vec3(0.0, 0.0, 1.0));
                    rot_z * rot_y * rot_x
                });
                let scale = get_transform("scale", &glm::scaling::<f32>);
                translation * rotation * scale
            }
            None => glm::identity(),
        };

        let material_index = material_names.iter().position(|n| object["material"] == *n).unwrap_or_default() as u32;

        Ok(Self { positions, normals, texcoords, indices, transform, material_index })
    }

    pub fn triangles(&self) -> Vec<Triangle> {
        self.indices
            .iter()
            .tuples::<(_, _, _)>()
            .map(|(&a, &b, &c)| Triangle {
                positions:      [self.positions[a as usize], self.positions[b as usize], self.positions[c as usize]],
                normals:        [self.normals[a as usize], self.normals[b as usize], self.normals[c as usize]],
                texcoords:      [self.texcoords[a as usize], self.texcoords[b as usize], self.texcoords[c as usize]],
                material_index: self.material_index,
            })
            .collect()
    }
}

impl Triangle {
    pub fn bounds(&self) -> BoundingBox {
        let [a, b, c] = self.positions;
        BoundingBox {
            min: glm::vec3(a.x.min(b.x).min(c.x), a.y.min(b.y).min(c.y), a.z.min(b.z).min(c.z)),
            max: glm::vec3(a.x.max(b.x).max(c.x), a.y.max(b.y).max(c.y), a.z.max(b.z).max(c.z)),
        }
    }

    pub fn into_uniform(self) -> TriangleUniform {
        TriangleUniform {
            positions:      std140::array![
                eruptrace_vk::std140::vec3(&self.positions[0]),
                eruptrace_vk::std140::vec3(&self.positions[1]),
                eruptrace_vk::std140::vec3(&self.positions[2]),
            ],
            normals:        std140::array![
                eruptrace_vk::std140::vec3(&self.normals[0]),
                eruptrace_vk::std140::vec3(&self.normals[1]),
                eruptrace_vk::std140::vec3(&self.normals[2]),
            ],
            texcoords:      std140::array![
                eruptrace_vk::std140::vec2(&self.texcoords[0]),
                eruptrace_vk::std140::vec2(&self.texcoords[1]),
                eruptrace_vk::std140::vec2(&self.texcoords[2]),
            ],
            material_index: std140::uint(self.material_index),
        }
    }
}
