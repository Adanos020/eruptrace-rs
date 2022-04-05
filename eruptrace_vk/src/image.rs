use crate::{command, AllocatedBuffer, VulkanContext};
use erupt::{vk, DeviceLoader};
use std::sync::{Arc, RwLock};
use vk_mem_erupt as vma;

#[derive(Clone)]
pub struct AllocatedImage {
    pub image: vk::Image,
    pub view: vk::ImageView,
    pub subresource_range: vk::ImageSubresourceRange,
    pub extent: vk::Extent3D,
    pub array_layers: u32,
    pub mip_levels: u32,

    allocator: Arc<RwLock<vma::Allocator>>,
    allocation: vma::Allocation,
    pub allocation_info: vma::AllocationInfo,
}

impl AllocatedImage {
    pub fn new(
        vk_ctx: VulkanContext,
        allocator: Arc<RwLock<vma::Allocator>>,
        image_info: vk::ImageCreateInfoBuilder,
        view_type: vk::ImageViewType,
        subresource_range: vk::ImageSubresourceRange,
    ) -> Self {
        let allocation_info = vma::AllocationCreateInfo {
            usage: vma::MemoryUsage::GpuOnly,
            flags: vma::AllocationCreateFlags::DEDICATED_MEMORY
                | vma::AllocationCreateFlags::MAPPED,
            ..Default::default()
        };

        let (image, allocation, allocation_info) = allocator
            .read()
            .unwrap()
            .create_image(
                &image_info.initial_layout(vk::ImageLayout::UNDEFINED),
                &allocation_info,
            )
            .expect("Cannot create image");

        let view_info = vk::ImageViewCreateInfoBuilder::new()
            .image(image)
            .view_type(view_type)
            .format(image_info.format)
            .components(vk::ComponentMapping {
                r: vk::ComponentSwizzle::IDENTITY,
                g: vk::ComponentSwizzle::IDENTITY,
                b: vk::ComponentSwizzle::IDENTITY,
                a: vk::ComponentSwizzle::IDENTITY,
            })
            .subresource_range(subresource_range);

        let view = unsafe {
            vk_ctx
                .device
                .create_image_view(&view_info, None)
                .expect("Cannot create image view")
        };

        Self {
            image,
            view,
            subresource_range,
            extent: image_info.extent,
            array_layers: image_info.array_layers,
            mip_levels: image_info.mip_levels,
            allocator,
            allocation,
            allocation_info,
        }
    }

    pub fn with_data<T: Sized>(
        vk_ctx: VulkanContext,
        allocator: Arc<RwLock<vma::Allocator>>,
        image_info: vk::ImageCreateInfoBuilder,
        view_type: vk::ImageViewType,
        range: vk::ImageSubresourceRange,
        data: &[T],
    ) -> Self {
        let this = Self::new(
            vk_ctx.clone(),
            allocator.clone(),
            image_info,
            view_type,
            range,
        );
        this.set_data(vk_ctx, data);
        allocator.read().unwrap().flush_allocation(
            &this.allocation,
            0,
            std::mem::size_of::<T>() * data.len(),
        );
        this
    }

    pub fn texture(
        vk_ctx: VulkanContext,
        allocator: Arc<RwLock<vma::Allocator>>,
        extent: vk::Extent3D,
        view_type: vk::ImageViewType,
        mip_levels: u32,
        array_layers: u32,
        texture_data: &[u8],
    ) -> Self {
        let image_info = vk::ImageCreateInfoBuilder::new()
            .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED)
            .format(vk::Format::R8G8B8A8_UNORM)
            .extent(extent)
            .mip_levels(mip_levels)
            .array_layers(array_layers)
            .samples(vk::SampleCountFlagBits::_1)
            .image_type(vk::ImageType::_2D);

        let range = vk::ImageSubresourceRangeBuilder::new()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .base_mip_level(0)
            .base_array_layer(0)
            .level_count(mip_levels)
            .layer_count(array_layers)
            .build();

        let this = Self::new(vk_ctx.clone(), allocator, image_info, view_type, range);
        this.set_data(vk_ctx, texture_data);
        this.allocator.read().unwrap().flush_allocation(
            &this.allocation,
            0,
            texture_data.len(),
        );
        this
    }

    pub fn color_attachment(
        vk_ctx: VulkanContext,
        allocator: Arc<RwLock<vma::Allocator>>,
        format: vk::Format,
        extent: vk::Extent3D,
        view_type: vk::ImageViewType,
        mip_levels: u32,
        array_layers: u32,
    ) -> Self {
        let image_info = vk::ImageCreateInfoBuilder::new()
            .usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .format(format)
            .extent(extent)
            .initial_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .array_layers(array_layers)
            .mip_levels(mip_levels)
            .samples(vk::SampleCountFlagBits::_1)
            .image_type(vk::ImageType::_2D);

        let range = vk::ImageSubresourceRangeBuilder::new()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .base_mip_level(0)
            .level_count(mip_levels)
            .base_array_layer(0)
            .layer_count(array_layers)
            .build();

        Self::new(vk_ctx, allocator, image_info, view_type, range)
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

    fn set_data<T>(&self, vk_ctx: VulkanContext, data: &[T]) {
        let image_buffer = {
            let buffer_info = vk::BufferCreateInfoBuilder::new()
                .usage(vk::BufferUsageFlags::TRANSFER_SRC)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);
            AllocatedBuffer::with_data(
                self.allocator.clone(),
                &buffer_info,
                vma::MemoryUsage::CpuOnly,
                data,
            )
        };

        command::immediate_submit(vk_ctx, |device, command_buffer| unsafe {
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
                    .src_access_mask(vk::AccessFlags::empty())
                    .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                    .image(self.image)
                    .subresource_range(self.subresource_range)],
            );
            device.cmd_copy_buffer_to_image(
                command_buffer,
                image_buffer.buffer,
                self.image,
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
                            .layer_count(self.array_layers)
                            .build(),
                    )
                    .image_extent(self.extent)],
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
                    .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                    .dst_access_mask(vk::AccessFlags::SHADER_READ)
                    .image(self.image)
                    .subresource_range(self.subresource_range)],
            );
        });

        image_buffer.destroy();
    }
}
