use crate::Triangle;
use nalgebra_glm as glm;

#[derive(Copy, Clone, Debug)]
pub struct BoundingBox {
    pub min: glm::Vec3,
    pub max: glm::Vec3,
}

#[derive(Copy, Clone, Debug)]
pub enum BihNodeData {
    Branch {
        clip_left: f32,
        clip_right: f32,
        child_left: usize,
        child_right: usize,
    },
    Leaf {
        triangle_index: usize,
        count: usize,
    },
}

#[repr(u32)]
#[derive(Copy, Clone, Debug)]
pub enum BihNodeType {
    X = 0,
    Y = 1,
    Z = 2,
    Leaf = 3,
}

#[derive(Copy, Clone, Debug)]
pub struct BihNode {
    ty: BihNodeType,
    data: BihNodeData,
}

#[derive(Clone, Debug)]
pub struct Bih(pub Vec<BihNode>);

enum Split {
    Leaf,
    Axis {
        ty: BihNodeType,
        middle: usize,
        left_box: BoundingBox,
        right_box: BoundingBox,
    },
}

impl BoundingBox {
    pub fn centre(&self) -> glm::Vec3 {
        (self.min + self.max) * 0.5
    }
}

impl Default for BihNode {
    fn default() -> Self {
        BihNode {
            ty: BihNodeType::Leaf,
            data: BihNodeData::Leaf {
                triangle_index: 0,
                count: 0,
            },
        }
    }
}

impl Bih {
    pub fn new(triangles: &mut [Triangle]) -> Self {
        if triangles.is_empty() {
            Self(vec![])
        } else {
            let all_triangles_addr = triangles.as_ptr() as usize;
            let bounds = calculate_bounds(triangles);
            let mut nodes = vec![BihNode::default()];
            nodes.reserve(2 * triangles.len());
            make_hierarchy(triangles, all_triangles_addr, bounds, 0, &mut nodes);
            nodes.shrink_to_fit();
            Self(nodes)
        }
    }
}

fn calculate_bounds(triangles: &[Triangle]) -> BoundingBox {
    triangles.iter().map(Triangle::bounds).fold(
        BoundingBox {
            min: glm::vec3(f32::MAX, f32::MAX, f32::MAX),
            max: glm::vec3(f32::MIN, f32::MIN, f32::MIN),
        },
        |s_bounds, t_bounds| BoundingBox {
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
        },
    )
}

fn make_hierarchy(
    triangles_part: &mut [Triangle],
    all_triangles_addr: usize,
    bounds: BoundingBox,
    current: usize,
    out_nodes: &mut Vec<BihNode>,
) {
    if !triangles_part.is_empty() {
        match split(triangles_part, bounds) {
            Split::Axis {
                ty,
                middle,
                left_box,
                right_box,
            } => {
                out_nodes.push(BihNode::default());
                out_nodes.push(BihNode::default());

                let child_left = out_nodes.len() - 2;
                let child_right = out_nodes.len() - 1;

                out_nodes[current].ty = ty;
                out_nodes[current].data = BihNodeData::Branch {
                    clip_left: left_box.max[ty as usize],
                    clip_right: right_box.min[ty as usize],
                    child_left,
                    child_right,
                };

                make_hierarchy(
                    &mut triangles_part[..middle],
                    all_triangles_addr,
                    left_box,
                    child_left,
                    out_nodes,
                );
                make_hierarchy(
                    &mut triangles_part[middle..],
                    all_triangles_addr,
                    right_box,
                    child_right,
                    out_nodes,
                );
            }
            Split::Leaf => {
                out_nodes[current].data = BihNodeData::Leaf {
                    triangle_index: (triangles_part.as_ptr() as usize - all_triangles_addr)
                        / std::mem::size_of::<Triangle>(),
                    count: triangles_part.len(),
                };
            }
        }
    }
}

fn split(triangles_part: &mut [Triangle], current_box: BoundingBox) -> Split {
    if triangles_part.len() > 1 {
        let mut axis_idx = choose_split_axis(current_box) as usize;
        for _ in 0..3 {
            let middle = triangles_part.iter_mut().partition_in_place(|t| {
                t.bounds().centre()[axis_idx] < current_box.centre()[axis_idx]
            });
            if middle > 0 && middle < triangles_part.len() - 1 {
                return Split::Axis {
                    ty: match axis_idx {
                        0 => BihNodeType::X,
                        1 => BihNodeType::Y,
                        2 => BihNodeType::Z,
                        _ => unreachable!(),
                    },
                    middle,
                    left_box: {
                        let mut bounds = current_box;
                        bounds.max[axis_idx] = triangles_part[..middle]
                            .iter()
                            .max_by(|t1, t2| {
                                t1.bounds().max[axis_idx].total_cmp(&t2.bounds().max[axis_idx])
                            })
                            .unwrap()
                            .bounds()
                            .min[axis_idx];
                        bounds
                    },
                    right_box: {
                        let mut bounds = current_box;
                        bounds.min[axis_idx] = triangles_part[middle..]
                            .iter()
                            .min_by(|t1, t2| {
                                t1.bounds().min[axis_idx].total_cmp(&t2.bounds().min[axis_idx])
                            })
                            .unwrap()
                            .bounds()
                            .max[axis_idx];
                        bounds
                    },
                };
            }
            axis_idx = (axis_idx + 1) % 3;
        }
    }
    Split::Leaf
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
