mod render_surface;
mod shaders;

use std::sync::{Arc, RwLock};

use erupt::{utils::surface, vk, DeviceLoader, EntryLoader, ExtendableFrom, InstanceLoader, SmallVec};
use erupt_bootstrap as vkb;
use eruptrace_deferred::DeferredRayTracer;
use eruptrace_pure::PureRayTracer;
use eruptrace_scene::{camera::Camera, CameraUniform, Scene, SceneBuffers};
use eruptrace_vk::{
    command,
    contexts::{FrameContext, PipelineContext, RenderContext, VulkanContext},
    debug::debug_callback,
    AllocatedBuffer,
};
use vk_mem_erupt as vma;
use winit::window::Window;

use crate::render_surface::RenderSurface;

pub struct App {
    _entry:                EntryLoader,
    debug_messenger:       Option<vk::DebugUtilsMessengerEXT>,
    instance:              Option<Arc<InstanceLoader>>,
    device:                Option<Arc<DeviceLoader>>,
    _device_meta:          vkb::DeviceMetadata,
    queue:                 vk::Queue,
    surface:               vk::SurfaceKHR,
    swapchain:             vkb::Swapchain,
    swapchain_image_views: SmallVec<vk::ImageView>,
    command_pool:          vk::CommandPool,
    frames:                Vec<FrameContext>,
    upload_fence:          vk::Fence,
    allocator:             Option<Arc<RwLock<vma::Allocator>>>,

    camera:           Camera,
    rt_camera_buffer: Option<AllocatedBuffer<CameraUniform>>,
    rt_scene_buffers: Option<SceneBuffers>,

    render_surface:      Option<RenderSurface>,
    pure_ray_tracer:     Option<PureRayTracer>,
    deferred_ray_tracer: Option<DeferredRayTracer>,
}

impl App {
    pub fn new(window: &Window, camera: Camera, scene: Scene) -> anyhow::Result<Self> {
        let entry = EntryLoader::new()?;
        let (instance, debug_messenger, instance_meta) = {
            let builder = vkb::InstanceBuilder::new()
                .request_api_version(1, 3)
                .require_surface_extensions(window)
                .expect("Cannot get surface extensions")
                .app_name("ErupTrace")?
                .validation_layers(vkb::ValidationLayers::Request)
                .request_debug_messenger(vkb::DebugMessenger::Custom {
                    callback:          debug_callback as _,
                    user_data_pointer: std::ptr::null_mut(),
                });
            let (instance, debug_messenger, instance_meta) = unsafe { builder.build(&entry)? };
            (Some(Arc::new(instance)), debug_messenger, instance_meta)
        };

        let surface = unsafe {
            surface::create_surface(instance.as_ref().unwrap(), window, None).expect("Cannot create surface")
        };

        let (device, device_meta, queue, queue_family) = {
            let graphics_present = vkb::QueueFamilyCriteria::graphics_present();
            let mut vulkan_1_3_features =
                vk::PhysicalDeviceVulkan13FeaturesBuilder::new().dynamic_rendering(true).synchronization2(true);
            let device_features = vk::PhysicalDeviceFeatures2Builder::new().extend_from(&mut vulkan_1_3_features);
            let device_builder = vkb::DeviceBuilder::new()
                .require_version(1, 3)
                .require_extension(vk::KHR_SWAPCHAIN_EXTENSION_NAME)
                .queue_family(graphics_present)
                .for_surface(surface)
                .require_features(&device_features);
            let (device, device_meta) = unsafe { device_builder.build(instance.as_ref().unwrap(), &instance_meta)? };
            let (queue, queue_family) = device_meta
                .device_queue(instance.as_ref().unwrap(), &device, graphics_present, 0)?
                .expect("Cannot get graphics present queue");
            (Some(Arc::new(device)), device_meta, queue, queue_family)
        };

        let format = {
            let surface_formats = unsafe {
                instance
                    .as_ref()
                    .unwrap()
                    .get_physical_device_surface_formats_khr(device_meta.physical_device(), surface, None)
                    .expect("Cannot get surface formats")
            };
            match *surface_formats.as_slice() {
                [f] if f.format == vk::Format::UNDEFINED => {
                    vk::SurfaceFormatKHR { format: vk::Format::B8G8R8A8_UNORM, color_space: f.color_space }
                }
                _ => *surface_formats
                    .iter()
                    .find(|f| {
                        let desirable_formats =
                            [vk::Format::R8G8B8A8_UNORM, vk::Format::B8G8R8A8_UNORM, vk::Format::A8B8G8R8_UNORM_PACK32];
                        desirable_formats.contains(&f.format)
                    })
                    .unwrap_or(&surface_formats[0]),
            }
        };

        let swapchain = {
            let mut swapchain_options = vkb::SwapchainOptions::default();
            swapchain_options.format_preference(&[format]);
            swapchain_options.present_mode_preference(&[vk::PresentModeKHR::MAILBOX_KHR, vk::PresentModeKHR::FIFO_KHR]);
            let [width, height]: [u32; 2] = window.inner_size().into();
            vkb::Swapchain::new(
                swapchain_options,
                surface,
                device_meta.physical_device(),
                device.as_ref().unwrap(),
                vk::Extent2D { width, height },
            )
        };

        let swapchain_image_views = SmallVec::new();

        let command_pool = {
            let create_info = vk::CommandPoolCreateInfoBuilder::new()
                .flags(vk::CommandPoolCreateFlags::TRANSIENT | vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
                .queue_family_index(queue_family);
            unsafe {
                device.as_ref().unwrap().create_command_pool(&create_info, None).expect("Cannot create command pool")
            }
        };

        let command_buffers = {
            let allocate_info = vk::CommandBufferAllocateInfoBuilder::new()
                .command_pool(command_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(swapchain.frames_in_flight() as u32);
            unsafe {
                device
                    .as_ref()
                    .unwrap()
                    .allocate_command_buffers(&allocate_info)
                    .expect("Cannot allocate command buffers")
            }
        };

        let frames: Vec<_> = command_buffers
            .iter()
            .map(|&command_buffer| unsafe {
                let create_info = vk::SemaphoreCreateInfoBuilder::default();
                FrameContext {
                    command_buffer,
                    complete: device
                        .as_ref()
                        .unwrap()
                        .create_semaphore(&create_info, None)
                        .expect("Cannot create frame semaphore"),
                }
            })
            .collect();

        let upload_fence = {
            let create_info = vk::FenceCreateInfoBuilder::new();
            unsafe { device.as_ref().unwrap().create_fence(&create_info, None).expect("Cannot create fence") }
        };

        let allocator = {
            let create_info = vma::AllocatorCreateInfo {
                physical_device:                 device_meta.physical_device(),
                device:                          device.as_ref().unwrap().clone(),
                instance:                        instance.as_ref().unwrap().clone(),
                flags:                           vma::AllocatorCreateFlags::empty(),
                preferred_large_heap_block_size: 0,
                frame_in_use_count:              0,
                heap_size_limits:                None,
            };
            let allocator = vma::Allocator::new(&create_info).expect("Cannot create memory allocator");
            Some(Arc::new(RwLock::new(allocator)))
        };

        let vk_ctx = VulkanContext {
            allocator: allocator.as_ref().unwrap().clone(),
            device: device.as_ref().unwrap().clone(),
            queue,
            command_pool,
            upload_fence,
        };

        let scene_meshes = scene.meshes.clone();
        let rt_scene_buffers = Some(scene.create_buffers(vk_ctx.clone()));
        let rt_camera_buffer = Some(camera.into_uniform().create_buffer(vk_ctx.allocator.clone()));

        let render_surface = Some(RenderSurface::new(
            vk_ctx.clone(),
            PipelineContext { surface_format: format },
            swapchain.frames_in_flight() as u32,
        )?);

        let pure_ray_tracer = Some(PureRayTracer::new(
            vk_ctx.clone(),
            vk::Extent2D { width: camera.img_size[0], height: camera.img_size[1] },
            rt_camera_buffer.as_ref().unwrap(),
            rt_scene_buffers.as_ref().unwrap(),
        ));

        let deferred_ray_tracer = Some(DeferredRayTracer::new(
            vk_ctx,
            camera,
            scene_meshes,
            rt_camera_buffer.as_ref().unwrap(),
            rt_scene_buffers.as_ref().unwrap(),
        )?);

        Ok(Self {
            _entry: entry,
            debug_messenger,
            instance,
            device,
            _device_meta: device_meta,
            queue,
            surface,
            swapchain,
            swapchain_image_views,
            command_pool,
            frames,
            upload_fence,
            allocator,
            camera,
            rt_camera_buffer,
            rt_scene_buffers,
            render_surface,
            pure_ray_tracer,
            deferred_ray_tracer,
        })
    }

    pub fn resize(&mut self, extent: vk::Extent2D) {
        let vk_ctx = VulkanContext {
            allocator:    self.allocator.as_ref().unwrap().clone(),
            device:       self.device.as_ref().unwrap().clone(),
            queue:        self.queue,
            command_pool: self.command_pool,
            upload_fence: self.upload_fence,
        };

        self.swapchain.update(extent);
        self.camera.img_size = [extent.width, extent.height];

        self.rt_camera_buffer.as_mut().unwrap().set_data(&[self.camera.into_uniform()]);
        self.render_surface.as_mut().unwrap().update_image_size(vk_ctx.clone(), extent);
        self.pure_ray_tracer.as_mut().unwrap().set_output_extent(extent);
        self.deferred_ray_tracer.as_mut().unwrap().update_output(vk_ctx.clone(), self.camera);

        self.pure_ray_tracer.as_mut().unwrap().render(vk_ctx, &self.render_surface.as_ref().unwrap().render_image);
    }

    pub fn render(&mut self) {
        let subresource_range = vk::ImageSubresourceRangeBuilder::new()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .base_mip_level(0)
            .level_count(1)
            .base_array_layer(0)
            .layer_count(1)
            .build();

        let acquired_frame = unsafe {
            self.swapchain.acquire(self.instance.as_ref().unwrap(), self.device.as_ref().unwrap(), u64::MAX).unwrap()
        };

        if acquired_frame.invalidate_images {
            for &image_view in self.swapchain_image_views.iter() {
                unsafe {
                    self.device.as_ref().unwrap().destroy_image_view(image_view, None);
                }
            }
            self.swapchain_image_views = self
                .swapchain
                .images()
                .iter()
                .map(|&img| unsafe {
                    let create_info = vk::ImageViewCreateInfoBuilder::new()
                        .image(img)
                        .view_type(vk::ImageViewType::_2D)
                        .format(self.swapchain.format().format)
                        .subresource_range(subresource_range);
                    self.device
                        .as_ref()
                        .unwrap()
                        .create_image_view(&create_info, None)
                        .expect("Cannot create swapchain image view")
                })
                .collect();
        }

        let in_flight = &self.frames[acquired_frame.frame_index];
        let swapchain_image = self.swapchain.images()[acquired_frame.image_index];
        let swapchain_image_view = self.swapchain_image_views[acquired_frame.image_index];

        let extent = self.swapchain.extent();

        unsafe {
            let begin_info =
                vk::CommandBufferBeginInfoBuilder::new().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            self.device
                .as_ref()
                .unwrap()
                .begin_command_buffer(in_flight.command_buffer, &begin_info)
                .expect("Cannot begin command buffer");
            command::set_scissor_and_viewport(self.device.as_ref().unwrap(), in_flight.command_buffer, extent);
        }

        unsafe {
            self.device.as_ref().unwrap().cmd_pipeline_barrier2(
                in_flight.command_buffer,
                &vk::DependencyInfoBuilder::new().image_memory_barriers(&[vk::ImageMemoryBarrier2Builder::new()
                    .src_stage_mask(vk::PipelineStageFlags2::NONE)
                    .dst_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
                    .src_access_mask(vk::AccessFlags2::NONE)
                    .dst_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
                    .old_layout(vk::ImageLayout::UNDEFINED)
                    .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .image(swapchain_image)
                    .subresource_range(subresource_range)]),
            );
        }

        let colour_attachment = vk::RenderingAttachmentInfoBuilder::new()
            .image_view(swapchain_image_view)
            .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .clear_value(vk::ClearValue { color: vk::ClearColorValue { float32: [0.0, 0.0, 0.0, 1.0] } })
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .resolve_image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

        let rendering_info = vk::RenderingInfoBuilder::new()
            .color_attachments(std::slice::from_ref(&colour_attachment))
            .layer_count(1)
            .render_area(vk::Rect2D { offset: Default::default(), extent });

        unsafe {
            self.device.as_ref().unwrap().cmd_begin_rendering(in_flight.command_buffer, &rendering_info);
        }

        self.render_surface.as_mut().unwrap().render(RenderContext {
            device:         self.device.as_ref().unwrap(),
            command_buffer: in_flight.command_buffer,
        });

        unsafe {
            self.device.as_ref().unwrap().cmd_end_rendering(in_flight.command_buffer);
        }

        unsafe {
            self.device.as_ref().unwrap().cmd_pipeline_barrier2(
                in_flight.command_buffer,
                &vk::DependencyInfoBuilder::new().image_memory_barriers(&[vk::ImageMemoryBarrier2Builder::new()
                    .src_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT_KHR)
                    .dst_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT_KHR)
                    .src_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE_KHR)
                    .dst_access_mask(
                        vk::AccessFlags2::COLOR_ATTACHMENT_READ_KHR | vk::AccessFlags2::COLOR_ATTACHMENT_WRITE_KHR,
                    )
                    .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                    .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .image(swapchain_image)
                    .subresource_range(subresource_range)]),
            );
            self.device
                .as_ref()
                .unwrap()
                .end_command_buffer(in_flight.command_buffer)
                .expect("Cannot end command buffer");
        }

        let wait_semaphore = vk::SemaphoreSubmitInfoBuilder::new()
            .semaphore(acquired_frame.ready)
            .stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT);
        let signal_semaphore = vk::SemaphoreSubmitInfoBuilder::new()
            .semaphore(in_flight.complete)
            .stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT);
        let command_buffer_info = vk::CommandBufferSubmitInfoBuilder::new().command_buffer(in_flight.command_buffer);
        let submit_info = vk::SubmitInfo2Builder::new()
            .wait_semaphore_infos(std::slice::from_ref(&wait_semaphore))
            .signal_semaphore_infos(std::slice::from_ref(&signal_semaphore))
            .command_buffer_infos(std::slice::from_ref(&command_buffer_info));
        unsafe {
            self.device
                .as_ref()
                .unwrap()
                .queue_submit2(self.queue, &[submit_info], acquired_frame.complete)
                .expect("Cannot submit commands to queue");
            self.swapchain
                .queue_present(
                    self.device.as_ref().unwrap(),
                    self.queue,
                    in_flight.complete,
                    acquired_frame.image_index,
                )
                .expect("Cannot present");
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        unsafe {
            let device = self.device.as_ref().unwrap();
            device.device_wait_idle().expect("Cannot wait idle");

            for &image_view in self.swapchain_image_views.iter() {
                device.destroy_image_view(image_view, None);
            }

            for frame in self.frames.iter() {
                device.destroy_semaphore(frame.complete, None);
            }

            device.destroy_fence(self.upload_fence, None);

            let prt_ref = self.pure_ray_tracer.as_ref().unwrap();
            prt_ref.destroy(device);
            self.pure_ray_tracer = None;

            let drt_ref = self.deferred_ray_tracer.as_ref().unwrap();
            drt_ref.destroy(device);
            self.deferred_ray_tracer = None;

            let rsf_ref = self.render_surface.as_ref().unwrap();
            rsf_ref.destroy(device);
            self.render_surface = None;

            self.rt_scene_buffers.as_ref().unwrap().destroy(device);
            self.rt_scene_buffers = None;

            self.rt_camera_buffer.as_ref().unwrap().destroy();
            self.rt_camera_buffer = None;

            device.destroy_command_pool(self.command_pool, None);

            self.swapchain.destroy(device);

            let mut alc_lock = self.allocator.as_ref().unwrap().write().unwrap();
            alc_lock.destroy();
            drop(alc_lock);
            self.allocator = None;

            self.instance.as_ref().unwrap().destroy_surface_khr(self.surface, None);

            device.destroy_device(None);
            self.device = None;
        }
        unsafe {
            if let Some(debug_messenger) = self.debug_messenger {
                if !debug_messenger.is_null() {
                    self.instance.as_ref().unwrap().destroy_debug_utils_messenger_ext(debug_messenger, None);
                }
            }

            self.instance.as_ref().unwrap().destroy_instance(None);
            self.instance = None;
        }
    }
}
