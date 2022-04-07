use itertools::Itertools;
use nalgebra_glm as glm;
use serde_json as json;

pub fn to_vec3(value: &json::Value) -> Option<glm::Vec3> {
    value.as_array().map(|arr| {
        let coords = arr.iter().map(|v| v.as_f64().unwrap() as f32).collect_vec();
        glm::make_vec3(&coords)
    })
}

pub fn to_vec2(value: &json::Value) -> Option<glm::Vec2> {
    value.as_array().map(|arr| {
        let coords = arr.iter().map(|v| v.as_f64().unwrap() as f32).collect_vec();
        glm::make_vec2(&coords)
    })
}
