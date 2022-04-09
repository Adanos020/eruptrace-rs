use erupt::DeviceLoader;
use eruptrace_scene::{CameraUniform, SceneBuffers};
use eruptrace_vk::{AllocatedBuffer, AllocatedImage, VulkanContext};

#[derive(Clone)]
pub struct LightingPass {}

impl LightingPass {
    pub fn new(
        vk_ctx: VulkanContext, camera_buffer: &AllocatedBuffer<CameraUniform>, scene_buffers: &SceneBuffers,
    ) -> Self {
        Self {}
    }

    pub fn destroy(&self, device: &DeviceLoader) {}

    pub fn render(&self, vk_ctx: VulkanContext, target: &AllocatedImage) {}
}
