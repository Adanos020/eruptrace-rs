use crate::Scene;

#[derive(Copy, Clone, Debug)]
pub struct BihBranch {
    clip_left: f32,
    clip_right: f32,
    child_left: usize,
    child_right: usize,
}

#[derive(Copy, Clone, Debug)]
pub enum BihNode {
    X(BihBranch),
    Y(BihBranch),
    Z(BihBranch),
    Leaf {
        mesh_index: usize,
        triangle_index: usize,
        count: usize,
    },
}

#[derive(Clone, Debug)]
pub struct Bih(pub Vec<BihNode>);

impl Bih {
    pub fn new(scene: &Scene) -> Self {
        unimplemented!()
    }
}
