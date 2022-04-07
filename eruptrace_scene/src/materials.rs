use serde_json as json;
use std140::repr_std140;

#[repr(u32)]
#[derive(Copy, Clone, Debug)]
pub enum MaterialType {
    Diffusive  = 0,
    Reflective = 1,
    Refractive = 2,
    Emitting   = 3,
}

#[derive(Copy, Clone, Debug)]
pub struct Material {
    pub material_type:    MaterialType,
    pub texture_index:    u32,
    pub normal_map_index: u32,
    /// The role of this parameter depends on the material type:
    /// - Diffusive: no function
    /// - Reflective: fuzz
    /// - Refractive: refractive index
    /// - Emitting: intensity
    pub parameter:        f32,
}

#[repr_std140]
#[derive(Copy, Clone, Debug)]
pub struct MaterialUniform {
    pub material_type:    std140::uint,
    pub texture_index:    std140::uint,
    pub normal_map_index: std140::uint,
    pub parameter:        std140::float,
}

impl From<&str> for MaterialType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "diffusive" => Self::Diffusive,
            "reflective" => Self::Reflective,
            "refractive" => Self::Refractive,
            "emitting" => Self::Emitting,
            _ => panic!("Invalid material type '{s}'."),
        }
    }
}

impl Material {
    pub fn from_json(
        object: &json::Value, texture_names: &[String], normal_map_names: &[String],
    ) -> anyhow::Result<Self> {
        let material_type = MaterialType::from(object["type"].as_str().unwrap_or_default());

        let texture_index = texture_names.iter().position(|n| object["texture"] == *n).unwrap_or_default() as u32;

        let normal_map_index =
            normal_map_names.iter().position(|n| object["normal_map"] == *n).unwrap_or_default() as u32;

        let parameter = match material_type {
            MaterialType::Diffusive => 1.0,
            MaterialType::Reflective => object["fuzz"].as_f64().unwrap_or(0.0) as f32,
            MaterialType::Refractive => object["index"].as_f64().unwrap_or(1.0) as f32,
            MaterialType::Emitting => object["intensity"].as_f64().unwrap_or(1.0) as f32,
        };

        Ok(Material { material_type, texture_index, normal_map_index, parameter })
    }

    pub fn into_uniform(self) -> MaterialUniform {
        MaterialUniform {
            material_type:    std140::uint(self.material_type as u32),
            texture_index:    std140::uint(self.texture_index),
            normal_map_index: std140::uint(self.normal_map_index),
            parameter:        std140::float(self.parameter),
        }
    }
}
