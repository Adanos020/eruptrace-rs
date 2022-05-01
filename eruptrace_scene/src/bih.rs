use nalgebra_glm as glm;
use std140::repr_std140;

use crate::Triangle;

#[derive(Copy, Clone, Debug)]
pub struct BoundingBox {
    pub min: glm::Vec3,
    pub max: glm::Vec3,
}

#[derive(Copy, Clone, Debug)]
pub enum BihNodeData {
    Branch { clip_left: f32, clip_right: f32, child_left: usize, child_right: usize },
    Leaf { triangle_index: usize, count: usize },
}

#[repr(u32)]
#[derive(Copy, Clone, Debug, enumn::N)]
pub enum BihNodeType {
    X    = 0,
    Y    = 1,
    Z    = 2,
    Leaf = 3,
}

#[derive(Copy, Clone, Debug)]
pub struct BihNode {
    pub ty:   BihNodeType,
    pub data: BihNodeData,
}

#[repr_std140]
#[derive(Copy, Clone, Debug)]
pub struct BihNodeUniform {
    pub node_type:   std140::uint,
    pub child_left:  std140::uint,
    pub child_right: std140::uint,
    pub clip_left:   std140::float,
    pub clip_right:  std140::float,
}

#[derive(Clone, Debug)]
pub struct Bih(pub Vec<BihNode>);

enum Split {
    Leaf,
    Axis {
        ty:         BihNodeType,
        middle:     usize,
        left_box:   BoundingBox,
        right_box:  BoundingBox,
        clip_left:  f32,
        clip_right: f32,
    },
}

impl BoundingBox {
    pub fn centre(&self) -> glm::Vec3 {
        (self.min + self.max) * 0.5
    }
}

impl Default for BihNode {
    fn default() -> Self {
        BihNode { ty: BihNodeType::Leaf, data: BihNodeData::Leaf { triangle_index: 0, count: 0 } }
    }
}

impl Bih {
    pub fn new(triangles: &mut [Triangle]) -> Self {
        let mut nodes = vec![BihNode::default()];
        if !triangles.is_empty() {
            let bounds = calculate_bounds(triangles);
            nodes.reserve(2 * triangles.len());
            make_hierarchy(triangles, 0, bounds, 0, &mut nodes);
            nodes.shrink_to_fit();
        }

        assert_eq!(triangles.len(), nodes.iter().fold(0, |c, n| match n.data {
            BihNodeData::Branch { .. } => c,
            BihNodeData::Leaf { count, .. } => c + count,
        }));
        // dbg!(&nodes);

        Self(nodes)
    }
}

impl BihNode {
    pub fn into_uniform(self) -> BihNodeUniform {
        match self.data {
            BihNodeData::Branch { clip_left, clip_right, child_left, child_right } => BihNodeUniform {
                node_type:   std140::uint(self.ty as u32),
                child_left:  std140::uint(child_left as u32),
                child_right: std140::uint(child_right as u32),
                clip_left:   std140::float(clip_left),
                clip_right:  std140::float(clip_right),
            },
            BihNodeData::Leaf { triangle_index, count } => BihNodeUniform {
                node_type:   std140::uint(self.ty as u32),
                child_left:  std140::uint(triangle_index as u32),
                child_right: std140::uint(count as u32),
                clip_left:   std140::float(0.0),
                clip_right:  std140::float(0.0),
            },
        }
    }
}

fn calculate_bounds(triangles: &[Triangle]) -> BoundingBox {
    triangles.iter().map(Triangle::bounds).fold(
        BoundingBox { min: glm::vec3(f32::MAX, f32::MAX, f32::MAX), max: glm::vec3(f32::MIN, f32::MIN, f32::MIN) },
        |s_bounds, t_bounds| BoundingBox {
            min: glm::vec3(
                s_bounds.min.x.min(t_bounds.min.x),
                s_bounds.min.y.min(t_bounds.min.y),
                s_bounds.min.z.min(t_bounds.min.z),
            ),
            max: glm::vec3(
                s_bounds.max.x.max(t_bounds.max.x),
                s_bounds.max.y.max(t_bounds.max.y),
                s_bounds.max.z.max(t_bounds.max.z),
            ),
        },
    )
}

fn make_hierarchy(
    triangles_part: &mut [Triangle],
    triangles_offset: usize,
    bounds: BoundingBox,
    current: usize,
    out_nodes: &mut Vec<BihNode>,
) {
    if !triangles_part.is_empty() {
        match split(triangles_part, bounds) {
            Split::Axis { ty, middle, left_box, right_box, clip_left, clip_right } => {
                out_nodes.push(BihNode::default());
                out_nodes.push(BihNode::default());

                let child_left = out_nodes.len() - 2;
                let child_right = out_nodes.len() - 1;

                out_nodes[current].ty = ty;
                out_nodes[current].data = BihNodeData::Branch { clip_left, clip_right, child_left, child_right };

                make_hierarchy(&mut triangles_part[..middle], triangles_offset, left_box, child_left, out_nodes);
                make_hierarchy(&mut triangles_part[middle..], triangles_offset + middle, right_box, child_right, out_nodes);
            }
            Split::Leaf => {
                out_nodes[current].data = BihNodeData::Leaf {
                    triangle_index: triangles_offset,
                    count:          triangles_part.len(),
                };
            }
        }
    }
}

fn split(triangles_part: &mut [Triangle], current_box: BoundingBox) -> Split {
    if triangles_part.len() > 1 {
        let mut axis_idx = choose_split_axis(current_box) as usize;
        for _ in 0..3 {
            let middle = triangles_part
                .iter_mut()
                .partition_in_place(|t| t.bounds().centre()[axis_idx] < current_box.centre()[axis_idx]);
            if (1..triangles_part.len()).contains(&middle) {
                let max_left = triangles_part[..middle]
                    .iter()
                    .map(|t| t.bounds())
                    .max_by(|b1, b2| b1.max[axis_idx].total_cmp(&b2.max[axis_idx]))
                    .unwrap();
                let min_right = triangles_part[middle..]
                    .iter()
                    .map(|t| t.bounds())
                    .min_by(|b1, b2| b1.min[axis_idx].total_cmp(&b2.min[axis_idx]))
                    .unwrap();
                return Split::Axis {
                    ty: BihNodeType::n(axis_idx as u32).unwrap(),
                    middle,
                    left_box: {
                        let mut bounds = current_box;
                        bounds.max[axis_idx] = max_left.min[axis_idx];
                        bounds
                    },
                    right_box: {
                        let mut bounds = current_box;
                        bounds.min[axis_idx] = min_right.max[axis_idx];
                        bounds
                    },
                    clip_left: max_left.max[axis_idx],
                    clip_right: min_right.min[axis_idx],
                };
            }
            axis_idx = (axis_idx + 1) % 3;
        }
    }
    Split::Leaf
}

fn choose_split_axis(bounds: BoundingBox) -> BihNodeType {
    let box_size: glm::Vec3 = bounds.max - bounds.min;
    if box_size.x > box_size.y && box_size.x > box_size.z {
        BihNodeType::X
    } else if box_size.y > box_size.z {
        BihNodeType::Y
    } else {
        BihNodeType::Z
    }
}
