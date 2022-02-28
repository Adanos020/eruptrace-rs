#![allow(clippy::no_effect)]

use eruptrace_scene::{
    materials::Material,
    shapes::{Mesh, Sphere},
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

#[derive(Clone)]
pub struct SceneBuffers {
    pub textures_image: Arc<ImmutableImage>,
    pub materials_buffer: Arc<ImmutableBuffer<[MaterialStd140]>>,
    pub shapes_buffer: Arc<ImmutableBuffer<[f32]>>,
    pub mesh_metas_buffer: Arc<ImmutableBuffer<[MeshMetaStd140]>>,
    pub mesh_data_buffer: Arc<ImmutableBuffer<[f32]>>,
}

#[repr_std140]
#[derive(Copy, Clone, Debug)]
pub struct MaterialStd140 {
    pub material_type: uint,
    pub texture_index: uint,
    pub parameter: float,
}

#[repr_std140]
#[derive(Copy, Clone, Debug)]
pub struct MeshMetaStd140 {
    pub material_index: uint,
    pub positions_start: uint,
    pub normals_start: uint,
    pub texcoords_start: uint,
    pub indices_start: uint,
}

pub type BufferFuture = CommandBufferExecFuture<NowFuture, PrimaryAutoCommandBuffer>;

pub fn make_scene_buffers(
    queue: Arc<Queue>,
    scene: Scene,
) -> (
    SceneBuffers,
    BufferFuture,
    BufferFuture,
    BufferFuture,
    BufferFuture,
    BufferFuture,
) {
    let n_textures = scene.texture_paths.len();
    let textures = get_texture_data(scene.texture_paths);
    let materials = get_material_data(scene.materials);
    let shapes = get_shapes_data(scene.spheres);
    let (mesh_metas, mesh_data) = get_mesh_data(scene.meshes);

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

    let (materials_buffer, materials_future) = ImmutableBuffer::from_iter(
        materials.into_iter(),
        BufferUsage::storage_buffer(),
        queue.clone(),
    )
    .expect("Cannot create buffer for materials.");

    let (shapes_buffer, shapes_future) = ImmutableBuffer::from_iter(
        shapes.into_iter(),
        BufferUsage::storage_buffer(),
        queue.clone(),
    )
    .expect("Cannot create buffer for shape data.");

    let (mesh_metas_buffer, mesh_metas_future) = ImmutableBuffer::from_iter(
        mesh_metas.into_iter(),
        BufferUsage::storage_buffer(),
        queue.clone(),
    )
    .expect("Cannot create buffer for mesh metas.");

    let (mesh_data_buffer, mesh_data_future) =
        ImmutableBuffer::from_iter(mesh_data.into_iter(), BufferUsage::storage_buffer(), queue)
            .expect("Cannot create buffer for mesh data.");

    (
        SceneBuffers {
            textures_image,
            materials_buffer,
            shapes_buffer,
            mesh_metas_buffer,
            mesh_data_buffer,
        },
        textures_future,
        materials_future,
        shapes_future,
        mesh_metas_future,
        mesh_data_future,
    )
}

fn get_texture_data(texture_paths: Vec<String>) -> Vec<u8> {
    texture_paths
        .iter()
        .map(|path| image::open(path).unwrap().into_rgba8())
        .flat_map(|texture| Vec::from(texture.as_bytes()))
        .collect()
}

fn get_material_data(materials: Vec<Material>) -> Vec<MaterialStd140> {
    let to_std140 = |mat: Material| MaterialStd140 {
        material_type: uint(mat.material_type as u32),
        texture_index: uint(mat.texture_index),
        parameter: float(mat.parameter),
    };
    materials.into_iter().map(to_std140).collect()
}

fn get_shapes_data(spheres: Vec<Sphere>) -> Vec<f32> {
    let mut data = Vec::with_capacity(spheres.len() * size_of::<Sphere>());

    data.push(spheres.len() as f32);
    for sphere in spheres.into_iter() {
        data.push(sphere.position.x);
        data.push(sphere.position.y);
        data.push(sphere.position.z);
        data.push(sphere.radius as f32);
        data.push(sphere.material_index as f32);
    }

    data
}

fn get_mesh_data(meshes: Vec<Mesh>) -> (Vec<MeshMetaStd140>, Vec<f32>) {
    let mut metas = Vec::with_capacity(meshes.len());
    let mut data = Vec::with_capacity(meshes.iter().map(|m| m.size_in_f32s()).sum());
    let mut curr_mesh_start = 0usize;

    // Number of meshes
    data.push(meshes.len() as f32);
    for mesh in meshes.into_iter() {
        let positions_size = mesh.positions.len() * 3;
        let normals_size = mesh.normals.len() * 3;
        let texcoords_size = mesh.texcoords.len() * 2;
        let indices_size = mesh.indices.len();

        let material_index = uint(mesh.material_index);
        let positions_start = uint(curr_mesh_start as u32);
        let normals_start = uint(positions_start.0 + positions_size as u32);
        let texcoords_start = uint(normals_start.0 + normals_size as u32);
        let indices_start = uint(texcoords_start.0 + texcoords_size as u32);

        metas.push(MeshMetaStd140 {
            material_index,
            positions_start,
            normals_start,
            texcoords_start,
            indices_start,
        });

        for position in mesh.positions.into_iter() {
            data.push(position.x);
            data.push(position.y);
            data.push(position.z);
        }
        for normal in mesh.normals.into_iter() {
            data.push(normal.x);
            data.push(normal.y);
            data.push(normal.z);
        }
        for texcoord in mesh.texcoords.into_iter() {
            data.push(texcoord.x);
            data.push(texcoord.y);
        }
        for index in mesh.indices.into_iter() {
            data.push(index as f32);
        }

        curr_mesh_start = indices_start.0 as usize + indices_size;
    }

    (metas, data)
}
