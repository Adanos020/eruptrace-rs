#![allow(clippy::no_effect)]

use eruptrace_scene::{materials::Material, primitives::Sphere, Scene};
use image::EncodableLayout;
use std::{mem::size_of, sync::Arc};
use std140::*;
use vulkano::{
    buffer::{BufferUsage, ImmutableBuffer},
    device::Queue,
    format::Format,
    image::{ImageDimensions, ImmutableImage, MipmapsCount},
    sync::GpuFuture,
};

#[repr_std140]
#[derive(Copy, Clone, Debug)]
pub struct MaterialStd140 {
    pub texture_index: uint,
    pub parameter: float,
}

pub type ShapesBuffer = Arc<ImmutableBuffer<[f32]>>;
pub type MaterialsBuffer = Arc<ImmutableBuffer<[MaterialStd140]>>;
pub type TexturesImage = Arc<ImmutableImage>;

pub fn make_scene_buffers(
    queue: Arc<Queue>,
    scene: Scene,
) -> (ShapesBuffer, MaterialsBuffer, TexturesImage) {
    let n_textures = scene.texture_paths.len();
    let shapes = get_shapes_data(scene.spheres);
    let materials = get_material_data(scene.materials);
    let textures = get_texture_data(scene.texture_paths);

    let (shapes_buffer, shapes_future) = ImmutableBuffer::from_iter(
        shapes.into_iter(),
        BufferUsage::storage_buffer(),
        queue.clone(),
    )
    .expect("Cannot create buffer for shapes.");

    let (materials_buffer, materials_future) = ImmutableBuffer::from_iter(
        materials.into_iter(),
        BufferUsage::storage_buffer(),
        queue.clone(),
    )
    .expect("Cannot create buffer for materials.");

    let (textures_image, textures_future) = ImmutableImage::from_iter(
        textures,
        ImageDimensions::Dim2d {
            width: 1024,
            height: 1024,
            array_layers: n_textures as u32,
        },
        MipmapsCount::One,
        Format::R8G8B8A8_UNORM,
        queue.clone(),
    )
    .expect("Cannot create textures image.");

    vulkano::sync::now(queue.device().clone())
        .join(shapes_future)
        .join(materials_future)
        .join(textures_future)
        .then_signal_fence_and_flush()
        .expect("Cannot flush.")
        .wait(None)
        .expect("Cannot wait.");

    (shapes_buffer, materials_buffer, textures_image)
}

fn get_shapes_data(spheres: Vec<Sphere>) -> Vec<f32> {
    let mut data = Vec::with_capacity(spheres.len() * size_of::<Sphere>());

    data.push(spheres.len() as f32);
    for sphere in spheres.iter() {
        data.push(sphere.position[0]);
        data.push(sphere.position[1]);
        data.push(sphere.position[2]);
        data.push(sphere.radius as f32);
        data.push(sphere.material_type as u32 as f32);
        data.push(sphere.material_index as f32);
    }

    data
}

fn get_material_data(materials: Vec<Material>) -> Vec<MaterialStd140> {
    let to_std140 = |mat: Material| {
        let texture_index = uint(mat.texture_index);
        let parameter = float(mat.parameter);
        MaterialStd140 {
            texture_index,
            parameter,
        }
    };
    materials.into_iter().map(to_std140).collect()
}

fn get_texture_data(texture_paths: Vec<String>) -> Vec<u8> {
    texture_paths
        .iter()
        .map(|path| image::open(path).unwrap().into_rgba8())
        .flat_map(|texture| Vec::from(texture.as_bytes()))
        .collect()
}
