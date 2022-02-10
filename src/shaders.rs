#![allow(clippy::needless_question_mark)]

pub mod rt_shaders {
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
