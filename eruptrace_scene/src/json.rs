use nalgebra_glm as glm;
use serde_json as json;

pub fn to_vec3(value: &json::Value) -> glm::Vec3 {
    let coords: Vec<f32> = value
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap() as f32)
        .collect();
    glm::make_vec3(&coords)
}

pub fn to_vec2(value: &json::Value) -> glm::Vec2 {
    let coords: Vec<f32> = value
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap() as f32)
        .collect();
    glm::make_vec2(&coords)
}
