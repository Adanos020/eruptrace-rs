use std::sync::Arc;
use vulkano::{
    buffer::{BufferUsage, ImmutableBuffer},
    device::{Device, Queue},
    sync::GpuFuture,
};

#[derive(Copy, Clone, Default, Debug)]
pub struct Vertex {
    pub position: [f32; 2],
}

vulkano::impl_vertex!(Vertex, position);

pub struct RenderSurface {
    pub vertex_buffer: Arc<ImmutableBuffer<[Vertex]>>,
    pub index_buffer: Arc<ImmutableBuffer<[u32]>>,
}

impl RenderSurface {
    pub fn new(device: Arc<Device>, queue: Arc<Queue>) -> Self {
        let vertices = [
            Vertex {
                position: [-1.0, -1.0],
            },
            Vertex {
                position: [1.0, -1.0],
            },
            Vertex {
                position: [1.0, 1.0],
            },
            Vertex {
                position: [-1.0, 1.0],
            },
        ];
        let indices = [0, 1, 2, 2, 3, 0];

        let (vertex_buffer, vb_future) =
            ImmutableBuffer::from_iter(vertices, BufferUsage::vertex_buffer(), queue.clone())
                .expect("Cannot create vertex buffer.");
        let (index_buffer, ib_future) =
            ImmutableBuffer::from_iter(indices, BufferUsage::index_buffer(), queue)
                .expect("Cannot create index buffer.");

        vulkano::sync::now(device)
            .join(vb_future)
            .join(ib_future)
            .then_signal_fence_and_flush()
            .expect("Cannot upload vertex and index buffers.")
            .wait(None)
            .expect("Cannot wait.");

        Self {
            vertex_buffer,
            index_buffer,
        }
    }
}
