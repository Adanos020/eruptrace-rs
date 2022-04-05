use crate::{
    render_surface::RenderSurface,
    {camera::CameraUniform, scene::SceneBuffers},
};
use erupt::DeviceLoader;
use eruptrace_scene::{camera::Camera, Scene};
use eruptrace_vk::{
    contexts::{PipelineContext, RenderContext},
    {AllocatedBuffer, VulkanContext},
};
use std::sync::{Arc, RwLock};
use vk_mem_erupt as vma;

pub mod camera;
pub mod render_surface;
pub mod scene;
pub mod shaders;

#[derive(Clone)]
pub struct PureRayTracer {
    camera_buffer: AllocatedBuffer<CameraUniform>,
    scene_buffers: SceneBuffers,
    render_surface: RenderSurface,
}

impl PureRayTracer {
    pub fn new(
        allocator: Arc<RwLock<vma::Allocator>>,
        vk_ctx: VulkanContext,
        pipeline_ctx: PipelineContext,
        camera: Camera,
        scene: Scene,
    ) -> Self {
        let camera_buffer = CameraUniform::from(camera).create_buffer(allocator.clone());
        let scene_buffers = SceneBuffers::create_buffers(allocator.clone(), vk_ctx.clone(), scene);
        let render_surface = RenderSurface::new(
            allocator,
            vk_ctx,
            pipeline_ctx,
            &camera_buffer,
            &scene_buffers,
        );

        Self {
            camera_buffer,
            scene_buffers,
            render_surface,
        }
    }

    pub fn destroy(&self, device: &DeviceLoader) {
        self.camera_buffer.destroy();
        self.scene_buffers.destroy(device);
        self.render_surface.destroy(device);
    }

    pub fn update_camera(&mut self, camera: Camera) {
        let camera_uniform = CameraUniform::from(camera);
        self.camera_buffer.set_data(&[camera_uniform]);
    }

    pub fn render(&self, ctx: RenderContext) {
        self.render_surface.render(ctx);
    }
}
