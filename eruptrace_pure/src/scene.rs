#![allow(clippy::no_effect)]

use eruptrace_scene::{
    materials::Material,
    primitives::{Sphere, Triangle},
    Scene,
};
use image::EncodableLayout;
use std::{mem::size_of, sync::Arc};
use std140::*;
use vulkano::{
    buffer::{BufferUsage, ImmutableBuffer},
    command_buffer::{CommandBufferExecFuture, PrimaryAutoCommandBuffer},
    device::Queue,
    format::Format,
    image::{ImageDimensions, ImmutableImage, MipmapsCount},
    sync::NowFuture,
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
pub type ShapesFuture = CommandBufferExecFuture<NowFuture, PrimaryAutoCommandBuffer>;
pub type MaterialsFuture = CommandBufferExecFuture<NowFuture, PrimaryAutoCommandBuffer>;
pub type TexturesFuture = CommandBufferExecFuture<NowFuture, PrimaryAutoCommandBuffer>;

pub fn make_scene_buffers(
    queue: Arc<Queue>,
    scene: Scene,
) -> (
    (ShapesBuffer, ShapesFuture),
    (MaterialsBuffer, MaterialsFuture),
    (TexturesImage, TexturesFuture),
) {
    let n_textures = scene.texture_paths.len();
    let shapes = get_shapes_data(scene.spheres, scene.triangles);
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
        queue,
    )
    .expect("Cannot create textures image.");

    (
        (shapes_buffer, shapes_future),
        (materials_buffer, materials_future),
        (textures_image, textures_future),
    )
}

fn get_shapes_data(spheres: Vec<Sphere>, triangles: Vec<Triangle>) -> Vec<f32> {
    let mut data = Vec::with_capacity({
        let n_spheres = spheres.len() * size_of::<Sphere>();
        let n_triangles = triangles.len() * size_of::<Triangle>();
        n_spheres + n_triangles
    });

    data.push(spheres.len() as f32);
    for sphere in spheres.into_iter() {
        data.push(sphere.position.x);
        data.push(sphere.position.y);
        data.push(sphere.position.z);
        data.push(sphere.radius as f32);
        data.push(sphere.material_type as u32 as f32);
        data.push(sphere.material_index as f32);
    }

    data.push(triangles.len() as f32);
    for triangle in triangles.into_iter() {
        for vertex in triangle.vertices {
            data.push(vertex.position.x);
            data.push(vertex.position.y);
            data.push(vertex.position.z);
            data.push(vertex.normal.x);
            data.push(vertex.normal.y);
            data.push(vertex.normal.z);
            data.push(vertex.texture_coordinate.x);
            data.push(vertex.texture_coordinate.y);
        }
        data.push(triangle.material_type as u32 as f32);
        data.push(triangle.material_index as f32);
    }

    data
}

fn get_material_data(materials: Vec<Material>) -> Vec<MaterialStd140> {
    let to_std140 = |mat: Material| MaterialStd140 {
        texture_index: uint(mat.texture_index),
        parameter: float(mat.parameter),
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
