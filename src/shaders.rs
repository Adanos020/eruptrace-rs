#![allow(clippy::needless_question_mark)]

pub mod cs {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "shaders/compute.comp"
    }
}
