use crate::VulkanContext;
use erupt::{vk, DeviceLoader};

pub fn immediate_submit<F>(vk_ctx: VulkanContext, execute_commands: F)
where
    F: FnOnce(&DeviceLoader, vk::CommandBuffer),
{
    let allocate_info = vk::CommandBufferAllocateInfoBuilder::new()
        .command_pool(vk_ctx.command_pool)
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_buffer_count(1);
    let begin_info = vk::CommandBufferBeginInfoBuilder::new()
        .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
    let command_buffers = unsafe {
        let command_buffer = vk_ctx
            .device
            .allocate_command_buffers(&allocate_info)
            .expect("Cannot allocate command buffers")[0];
        vk_ctx
            .device
            .begin_command_buffer(command_buffer, &begin_info)
            .expect("Cannot begin command buffer");
        execute_commands(&vk_ctx.device, command_buffer);
        vk_ctx
            .device
            .end_command_buffer(command_buffer)
            .expect("Cannot end command buffer");
        [command_buffer]
    };

    let submit_info = vk::SubmitInfoBuilder::new().command_buffers(&command_buffers);
    unsafe {
        vk_ctx
            .device
            .queue_submit(vk_ctx.queue, &[submit_info], vk_ctx.upload_fence)
            .expect("Cannot submit queue");
        vk_ctx
            .device
            .wait_for_fences(&[vk_ctx.upload_fence], true, 9999999999)
            .expect("Cannot wait for upload fence");
        vk_ctx
            .device
            .reset_fences(&[vk_ctx.upload_fence])
            .expect("Cannot reset upload fence");
        vk_ctx
            .device
            .reset_command_pool(vk_ctx.command_pool, vk::CommandPoolResetFlags::empty())
            .expect("Cannot reset command pool");
    }
}
