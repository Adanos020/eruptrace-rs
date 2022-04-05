use crate::{
    bih::BoundingBox,
    json::{to_vec2, to_vec3},
};
use itertools::Itertools;
use nalgebra_glm as glm;
use serde_json as js;

#[derive(Clone, Debug)]
pub struct Triangle {
    pub positions: [glm::Vec3; 3],
    pub normals: [glm::Vec3; 3],
    pub texcoords: [glm::Vec2; 3],
    pub material_index: u32,
}

#[derive(Clone, Debug)]
pub struct Mesh {
    pub positions: Vec<glm::Vec3>,
    pub normals: Vec<glm::Vec3>,
    pub texcoords: Vec<glm::Vec2>,
    pub indices: Vec<u32>,
    pub transform: glm::Mat4x4,
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

        let transform: glm::Mat4x4 = match object["transform"].as_object() {
            Some(object) => {
                let get_transform = |name, func: &dyn Fn(&glm::Vec3) -> glm::Mat4x4| match object
                    .get(name)
                    .and_then(|a| a.as_array())
                {
                    Some(arr) => {
                        let vector = glm::vec3(
                            arr[0].as_f64().unwrap() as f32,
                            arr[1].as_f64().unwrap() as f32,
                            arr[2].as_f64().unwrap() as f32,
                        );
                        func(&vector)
                    }
                    None => glm::identity(),
                };
                let translation = get_transform("position", &glm::translation::<f32>);
                let rotation = get_transform("rotation", &|vec| {
                    let rot_x = glm::rotation(vec[0].to_radians(), &glm::vec3(1.0, 0.0, 0.0));
                    let rot_y = glm::rotation(vec[1].to_radians(), &glm::vec3(0.0, 1.0, 0.0));
                    let rot_z = glm::rotation(vec[2].to_radians(), &glm::vec3(0.0, 0.0, 1.0));
                    rot_z * rot_y * rot_x
                });
                let scale = get_transform("scale", &glm::scaling::<f32>);
                translation * rotation * scale
            }
            None => glm::identity(),
        };

        let material_index = material_names
            .iter()
            .position(|n| object["material"] == *n)
            .unwrap_or_default() as u32;

        Ok(Self {
            positions,
            normals,
            texcoords,
            indices,
            transform,
            material_index,
        })
    }

    pub fn triangles(&self) -> Vec<Triangle> {
        self.indices
            .iter()
            .tuples::<(_, _, _)>()
            .map(|(&a, &b, &c)| Triangle {
                positions: [
                    self.positions[a as usize],
                    self.positions[b as usize],
                    self.positions[c as usize],
                ],
                normals: [
                    self.normals[a as usize],
                    self.normals[b as usize],
                    self.normals[c as usize],
                ],
                texcoords: [
                    self.texcoords[a as usize],
                    self.texcoords[b as usize],
                    self.texcoords[c as usize],
                ],
                material_index: self.material_index,
            })
            .collect()
    }
}

impl Triangle {
    pub fn bounds(&self) -> BoundingBox {
        let [a, b, c] = self.positions;
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
