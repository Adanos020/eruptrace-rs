use nalgebra_glm as glm;

#[derive(Copy, Clone, Debug)]
pub struct Sphere {
    pub position: glm::TVec3<f32>,
    pub radius: f32,
    pub material_index: u32,
}

#[derive(Clone, Debug)]
pub struct Mesh {
    pub positions: Vec<glm::TVec3<f32>>,
    pub normals: Vec<glm::TVec3<f32>>,
    pub texcoords: Vec<glm::TVec2<f32>>,
    pub indices: Vec<u32>,
    pub material_index: u32,
}

impl Mesh {
    pub fn size_in_f32s(&self) -> usize {
        self.positions.len() * 3
            + self.normals.len() * 3
            + self.texcoords.len() * 2
            + self.indices.len()
            + 1
    }
}
