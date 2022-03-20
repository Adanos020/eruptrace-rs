use crate::{command, AllocatedBuffer, VulkanContext};
use erupt::{vk, DeviceLoader};
use std::sync::{Arc, RwLock};
use vk_mem_erupt as vma;

#[derive(Clone)]
pub struct AllocatedImage {
    pub image: vk::Image,
    pub view: vk::ImageView,

    allocator: Arc<RwLock<vma::Allocator>>,
    allocation: vma::Allocation,
    pub allocation_info: vma::AllocationInfo,
}

impl AllocatedImage {
    pub fn with_data<T: Sized>(
        vk_ctx: VulkanContext,
        allocator: Arc<RwLock<vma::Allocator>>,
        format: vk::Format,
        extent: vk::Extent3D,
        view_type: vk::ImageViewType,
        mip_levels: u32,
        array_layers: u32,
        data: &[T],
    ) -> vma::Result<Self> {
        let image_buffer = {
            let buffer_info = vk::BufferCreateInfoBuilder::new()
                .usage(vk::BufferUsageFlags::TRANSFER_SRC)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);
            let allocation_info = vma::AllocationCreateInfo {
                usage: vma::MemoryUsage::CpuOnly,
                flags: vma::AllocationCreateFlags::DEDICATED_MEMORY
                    | vma::AllocationCreateFlags::MAPPED,
                ..Default::default()
            };
            AllocatedBuffer::with_data(allocator.clone(), &buffer_info, allocation_info, data)
                .expect("Cannot create temporary buffer for image data")
        };

        let (image, allocation, allocation_info) = {
            let image_info = vk::ImageCreateInfoBuilder::new()
                .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED)
                .format(format)
                .extent(extent)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .array_layers(array_layers)
                .mip_levels(mip_levels)
                .samples(vk::SampleCountFlagBits::_1)
                .image_type(vk::ImageType::_2D);
            let allocation_info = vma::AllocationCreateInfo {
                usage: vma::MemoryUsage::GpuOnly,
                flags: vma::AllocationCreateFlags::DEDICATED_MEMORY
                    | vma::AllocationCreateFlags::MAPPED,
                ..Default::default()
            };
            allocator
                .read()
                .unwrap()
                .create_image(&image_info, &allocation_info)
                .expect("Cannot create image")
        };

        let range = vk::ImageSubresourceRangeBuilder::new()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .base_mip_level(0)
            .level_count(mip_levels)
            .base_array_layer(0)
            .layer_count(array_layers)
            .build();

        command::immediate_submit(vk_ctx.clone(), |device, command_buffer| unsafe {
            device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[vk::ImageMemoryBarrierBuilder::new()
                    .old_layout(vk::ImageLayout::UNDEFINED)
                    .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                    .image(image)
                    .subresource_range(range)
                    .src_access_mask(vk::AccessFlags::empty())
                    .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)],
            );
            device.cmd_copy_buffer_to_image(
                command_buffer,
                image_buffer.buffer,
                image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[vk::BufferImageCopyBuilder::new()
                    .buffer_offset(0)
                    .buffer_row_length(0)
                    .buffer_image_height(0)
                    .image_subresource(
                        vk::ImageSubresourceLayersBuilder::new()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .mip_level(0)
                            .base_array_layer(0)
                            .layer_count(array_layers)
                            .build(),
                    )
                    .image_extent(extent)],
            );
            device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[vk::ImageMemoryBarrierBuilder::new()
                    .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                    .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                    .image(image)
                    .subresource_range(range)
                    .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                    .dst_access_mask(vk::AccessFlags::SHADER_READ)],
            );
        });

        image_buffer.destroy();

        let view = {
            let view_info = vk::ImageViewCreateInfoBuilder::new()
                .image(image)
                .view_type(view_type)
                .format(format)
                .components(vk::ComponentMapping {
                    r: vk::ComponentSwizzle::IDENTITY,
                    g: vk::ComponentSwizzle::IDENTITY,
                    b: vk::ComponentSwizzle::IDENTITY,
                    a: vk::ComponentSwizzle::IDENTITY,
                })
                .subresource_range(range);
            unsafe {
                vk_ctx
                    .device
                    .create_image_view(&view_info, None)
                    .expect("Cannot create image view")
            }
        };

        allocator.read().unwrap().flush_allocation(
            &allocation,
            0,
            std::mem::size_of::<T>() * data.len(),
        );

        Ok(Self {
            image,
            view,
            allocator,
            allocation,
            allocation_info,
        })
    }

    pub fn destroy(&self, device: &DeviceLoader) {
        unsafe {
            device.destroy_image_view(self.view, None);
        }
        self.allocator
            .read()
            .unwrap()
            .destroy_image(self.image, &self.allocation);
    }
}
