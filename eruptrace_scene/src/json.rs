use nalgebra_glm as glm;
use serde_json as json;

pub fn to_vec3(value: &json::Value) -> glm::TVec3<f32> {
    let coords = value.as_array().unwrap();
    let coords: Vec<f32> = coords.iter().map(|v| v.as_f64().unwrap() as f32).collect();
    glm::make_vec3(&coords)
}

pub fn to_vec2(value: &json::Value) -> glm::TVec2<f32> {
    let coords = value.as_array().unwrap();
    let coords: Vec<f32> = coords.iter().map(|v| v.as_f64().unwrap() as f32).collect();
    glm::make_vec2(&coords)
}
