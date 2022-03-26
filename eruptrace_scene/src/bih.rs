use crate::{Mesh, Scene, Triangle};
use nalgebra_glm as glm;

#[derive(Copy, Clone, Debug)]
pub struct BoundingBox {
    pub min: glm::Vec3,
    pub max: glm::Vec3,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub enum BihNodeData {
    Branch {
        clip_left: f32,
        clip_right: f32,
        child_left: usize,
        child_right: usize,
    },
    Leaf {
        mesh_index: usize,
        triangle_index: usize,
        count: usize,
    },
}

#[repr(u32)]
#[derive(Copy, Clone, Debug)]
pub enum BihNodeType {
    X, Y, Z, Leaf,
}

#[derive(Copy, Clone, Debug)]
pub struct BihNode {
    ty: BihNodeType,
    data: BihNodeData,
}

#[derive(Clone, Debug)]
pub struct Bih(pub Vec<BihNode>);

impl Bih {
    pub fn new(scene: &Scene) -> Self {
        if scene.triangles.is_empty() {
            Self(vec![])
        } else {
            let bounds = calculate_bounds(&scene.triangles);
            let mut nodes = Vec::with_capacity(2 * scene.triangles.len());
            make_hierarchy(&scene.triangles, &scene.triangles, bounds, 0, &mut nodes);
            nodes.shrink_to_fit();
            Self(nodes)
        }
    }
}

fn calculate_bounds(triangles: &[Triangle]) -> BoundingBox {
    triangles
        .iter()
        .map(Triangle::bounds)
        .fold(
            BoundingBox {
                min: glm::vec3(f32::MAX, f32::MAX, f32::MAX),
                max: glm::vec3(f32::MIN, f32::MIN, f32::MIN),
            },
            |mut s_bounds, t_bounds| BoundingBox {
                min: glm::vec3(
                    s_bounds.min[0].min(t_bounds.min[0]),
                    s_bounds.min[1].min(t_bounds.min[1]),
                    s_bounds.min[2].min(t_bounds.min[2]),
                ),
                max: glm::vec3(
                    s_bounds.max[0].max(t_bounds.max[0]),
                    s_bounds.max[1].max(t_bounds.max[1]),
                    s_bounds.max[2].max(t_bounds.max[2]),
                ),
            }
        )
}

fn make_hierarchy(
    triangles_part: &[Triangle],
    triangles: &[Triangle],
    bounds: BoundingBox,
    current: usize,
    out_nodes: &mut Vec<BihNode>,
) {

}

fn split() {

}

fn choose_split_axis(bounds: BoundingBox) -> BihNodeType {
    let box_size = bounds.max - bounds.min;
    if box_size[0] > box_size[1] && box_size[0] > box_size[2] {
        BihNodeType::X
    } else if box_size[1] > box_size[2] {
        BihNodeType::Y
    } else {
        BihNodeType::Z
    }
}
