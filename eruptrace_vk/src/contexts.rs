use std::sync::{Arc, RwLock};

use erupt::{vk, DeviceLoader};
use vk_mem_erupt as vma;

#[derive(Clone)]
pub struct VulkanContext {
    pub device:       Arc<DeviceLoader>,
    pub allocator:    Arc<RwLock<vma::Allocator>>,
    pub queue:        vk::Queue,
    pub command_pool: vk::CommandPool,
    pub upload_fence: vk::Fence,
}

#[derive(Copy, Clone)]
pub struct PipelineContext {
    pub surface_format: vk::SurfaceFormatKHR,
}

#[derive(Copy, Clone)]
pub struct FrameContext {
    pub command_buffer: vk::CommandBuffer,
    pub complete:       vk::Semaphore,
}

#[derive(Copy, Clone)]
pub struct RenderContext<'a> {
    pub device:         &'a DeviceLoader,
    pub command_buffer: vk::CommandBuffer,
}
