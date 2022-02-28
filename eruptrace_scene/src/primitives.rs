use nalgebra_glm as glm;

#[derive(Copy, Clone, Debug)]
pub struct PolygonVertex {
    pub position: glm::TVec3<f32>,
    pub normal: glm::TVec3<f32>,
    pub texture_coordinate: glm::TVec2<f32>,
}

#[derive(Copy, Clone, Debug)]
pub struct Sphere {
    pub position: glm::TVec3<f32>,
    pub radius: f32,
    pub material_index: u32,
}

#[derive(Copy, Clone, Debug)]
pub struct Triangle {
    pub vertices: [PolygonVertex; 3],
    pub material_index: u32,
}
