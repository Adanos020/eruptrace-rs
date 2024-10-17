use std::sync::{Arc, RwLock};

use erupt::{vk, DeviceLoader};
use vk_mem_3_erupt as vma;

use crate::{command, AllocatedBuffer, VulkanContext};

#[derive(Clone)]
pub struct AllocatedImage {
    pub image:             vk::Image,
    pub view:              vk::ImageView,
    pub subresource_range: vk::ImageSubresourceRange,
    pub extent:            vk::Extent3D,
    pub array_layers:      u32,
    pub mip_levels:        u32,

    allocator:           Arc<RwLock<vma::Allocator>>,
    allocation:          vma::Allocation,
    pub allocation_info: vma::AllocationInfo,
}

impl AllocatedImage {
    pub fn new(
        vk_ctx: VulkanContext,
        image_info: vk::ImageCreateInfoBuilder,
        layout: Option<vk::ImageLayout>,
        view_type: vk::ImageViewType,
        subresource_range: vk::ImageSubresourceRange,
    ) -> Self {
        let allocation_info =
            vma::AllocationCreateInfo { usage: vma::MemoryUsage::AutoPreferDevice, ..Default::default() };

        let (image, allocation, allocation_info) = vk_ctx
            .allocator
            .read()
            .unwrap()
            .create_image(&image_info.initial_layout(vk::ImageLayout::UNDEFINED), &allocation_info)
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

        let view = unsafe { vk_ctx.device.create_image_view(&view_info, None).expect("Cannot create image view") };

        if let Some(layout) = layout {
            command::immediate_submit(vk_ctx.clone(), |device, command_buffer| unsafe {
                device.cmd_pipeline_barrier2(
                    command_buffer,
                    &vk::DependencyInfoBuilder::new().image_memory_barriers(&[vk::ImageMemoryBarrier2Builder::new()
                        .old_layout(vk::ImageLayout::UNDEFINED)
                        .new_layout(layout)
                        .image(image)
                        .subresource_range(subresource_range)]),
                );
            });
        }

        Self {
            image,
            view,
            subresource_range,
            extent: image_info.extent,
            array_layers: image_info.array_layers,
            mip_levels: image_info.mip_levels,
            allocator: vk_ctx.allocator,
            allocation,
            allocation_info,
        }
    }

    pub fn with_data<T: Sized>(
        vk_ctx: VulkanContext,
        image_info: vk::ImageCreateInfoBuilder,
        view_type: vk::ImageViewType,
        range: vk::ImageSubresourceRange,
        data: &[T],
    ) -> Self {
        let this = Self::new(vk_ctx.clone(), image_info, None, view_type, range);
        this.set_data(vk_ctx.clone(), vk::Offset3D::default(), data);
        vk_ctx.allocator.read().unwrap().flush_allocation(&this.allocation, 0, std::mem::size_of::<T>() * data.len());
        this
    }

    pub fn texture(
        vk_ctx: VulkanContext,
        format: vk::Format,
        extent: vk::Extent3D,
        view_type: vk::ImageViewType,
        mip_levels: u32,
        array_layers: u32,
    ) -> Self {
        let image_info = vk::ImageCreateInfoBuilder::new()
            .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED)
            .format(format)
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

        Self::new(vk_ctx, image_info, Some(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL), view_type, range)
    }

    pub fn texture_with_data<T>(
        vk_ctx: VulkanContext,
        format: vk::Format,
        extent: vk::Extent3D,
        view_type: vk::ImageViewType,
        mip_levels: u32,
        array_layers: u32,
        texture_data: &[T],
    ) -> Self {
        let this = Self::texture(vk_ctx.clone(), format, extent, view_type, mip_levels, array_layers);
        this.set_data(vk_ctx.clone(), vk::Offset3D::default(), texture_data);
        vk_ctx.allocator.read().unwrap().flush_allocation(&this.allocation, 0, texture_data.len());
        this
    }

    pub fn color_attachment(
        vk_ctx: VulkanContext,
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

        Self::new(vk_ctx, image_info, Some(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL), view_type, range)
    }

    pub fn depth_buffer(vk_ctx: VulkanContext, extent: vk::Extent3D) -> Self {
        let image_info = vk::ImageCreateInfoBuilder::new()
            .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
            .format(vk::Format::D32_SFLOAT)
            .extent(extent)
            .array_layers(1)
            .mip_levels(1)
            .samples(vk::SampleCountFlagBits::_1)
            .image_type(vk::ImageType::_2D);

        let range = vk::ImageSubresourceRangeBuilder::new()
            .aspect_mask(vk::ImageAspectFlags::DEPTH)
            .base_mip_level(0)
            .level_count(1)
            .base_array_layer(0)
            .layer_count(1)
            .build();

        Self::new(
            vk_ctx,
            image_info,
            Some(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL),
            vk::ImageViewType::_2D,
            range,
        )
    }

    pub fn gbuffer(
        vk_ctx: VulkanContext,
        format: vk::Format,
        extent: vk::Extent3D,
        view_type: vk::ImageViewType,
        mip_levels: u32,
        array_layers: u32,
    ) -> Self {
        let image_info = vk::ImageCreateInfoBuilder::new()
            .usage(
                vk::ImageUsageFlags::COLOR_ATTACHMENT
                    | vk::ImageUsageFlags::TRANSFER_DST
                    | vk::ImageUsageFlags::SAMPLED,
            )
            .format(format)
            .extent(extent)
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

        Self::new(vk_ctx, image_info, Some(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL), view_type, range)
    }

    pub fn destroy(&self, device: &DeviceLoader) {
        unsafe {
            device.destroy_image_view(self.view, None);
        }
        self.allocator.read().unwrap().destroy_image(self.image, &self.allocation);
    }

    pub fn set_data<T>(&self, vk_ctx: VulkanContext, image_offset: vk::Offset3D, data: &[T]) {
        let image_buffer = {
            let buffer_info = vk::BufferCreateInfoBuilder::new()
                .usage(vk::BufferUsageFlags::TRANSFER_SRC)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);
            AllocatedBuffer::with_data(self.allocator.clone(), &buffer_info, vma::MemoryUsage::AutoPreferHost, data)
        };

        command::immediate_submit(vk_ctx, |device, command_buffer| unsafe {
            device.cmd_pipeline_barrier2(
                command_buffer,
                &vk::DependencyInfoBuilder::new().image_memory_barriers(&[vk::ImageMemoryBarrier2Builder::new()
                    .src_stage_mask(vk::PipelineStageFlags2::TOP_OF_PIPE)
                    .dst_stage_mask(vk::PipelineStageFlags2::TRANSFER)
                    .src_access_mask(vk::AccessFlags2::empty())
                    .dst_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
                    .old_layout(vk::ImageLayout::UNDEFINED)
                    .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                    .image(self.image)
                    .subresource_range(self.subresource_range)]),
            );
            device.cmd_copy_buffer_to_image2(
                command_buffer,
                &vk::CopyBufferToImageInfo2Builder::new()
                    .src_buffer(image_buffer.buffer)
                    .dst_image(self.image)
                    .dst_image_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                    .regions(&[vk::BufferImageCopy2Builder::new()
                        .buffer_offset(0)
                        .buffer_row_length(0)
                        .buffer_image_height(0)
                        .image_offset(image_offset)
                        .image_subresource(
                            vk::ImageSubresourceLayersBuilder::new()
                                .aspect_mask(vk::ImageAspectFlags::COLOR)
                                .mip_level(0)
                                .base_array_layer(0)
                                .layer_count(self.array_layers)
                                .build(),
                        )
                        .image_extent(self.extent)]),
            );
            device.cmd_pipeline_barrier2(
                command_buffer,
                &vk::DependencyInfoBuilder::new().image_memory_barriers(&[vk::ImageMemoryBarrier2Builder::new()
                    .src_stage_mask(vk::PipelineStageFlags2::TRANSFER)
                    .dst_stage_mask(vk::PipelineStageFlags2::FRAGMENT_SHADER)
                    .src_access_mask(vk::AccessFlags2::NONE)
                    .dst_access_mask(vk::AccessFlags2::SHADER_SAMPLED_READ)
                    .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                    .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                    .image(self.image)
                    .subresource_range(self.subresource_range)]),
            );
        });

        image_buffer.destroy();
    }

    pub fn copy_to_buffer<T>(&self, vk_ctx: VulkanContext, buffer: &AllocatedBuffer<T>) {
        command::immediate_submit(vk_ctx, |device, command_buffer| unsafe {
            device.cmd_pipeline_barrier2(
                command_buffer,
                &vk::DependencyInfoBuilder::new().image_memory_barriers(&[vk::ImageMemoryBarrier2Builder::new()
                    .src_stage_mask(vk::PipelineStageFlags2::TOP_OF_PIPE)
                    .dst_stage_mask(vk::PipelineStageFlags2::TRANSFER)
                    .src_access_mask(vk::AccessFlags2::empty())
                    .dst_access_mask(vk::AccessFlags2::TRANSFER_READ)
                    .old_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
                    .new_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
                    .image(self.image)
                    .subresource_range(self.subresource_range)]),
            );
            device.cmd_copy_image_to_buffer2(
                command_buffer,
                &vk::CopyImageToBufferInfo2Builder::new()
                    .src_image(self.image)
                    .src_image_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
                    .dst_buffer(buffer.buffer)
                    .regions(&[vk::BufferImageCopy2Builder::new()
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
                        .image_extent(self.extent)]),
            );
        });
    }
}
