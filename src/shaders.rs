#![allow(clippy::needless_question_mark)]

pub mod shader_raytracer {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "shaders/raytracer.comp"
    }
}

pub mod shaders_image {
    vulkano_shaders::shader! {
        shaders: {
            vertex: {
                ty: "vertex",
                path: "shaders/image.vert"
            },
            fragment: {
                ty: "fragment",
                path: "shaders/image.frag"
            }
        }
    }
}
