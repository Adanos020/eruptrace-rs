pub mod camera;
pub mod render_surface;
pub mod scene;
pub mod shaders;

use crate::{
    camera::CameraUniform, render_surface::RenderSurface, scene::make_scene_buffers,
    shaders::rt_shaders,
};
use eruptrace_scene::{camera::Camera, Scene};
use std::sync::Arc;
use vulkano::{
    command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, SubpassContents},
    device::{physical::PhysicalDevice, Device, DeviceExtensions, Features, Queue},
    image::{view::ImageView, ImageAccess, ImageUsage, SwapchainImage},
    instance::Instance,
    pipeline::graphics::viewport::Viewport,
    render_pass::{Framebuffer, RenderPass},
    swapchain::{AcquireError, Swapchain, SwapchainCreationError},
    sync::{FlushError, GpuFuture},
    Version,
};
use vulkano_win::VkSurfaceBuild;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

pub fn run_app(mut camera: Camera, scene: Scene) {
    let instance = {
        let extensions = vulkano_win::required_extensions();
        Instance::new(None, Version::V1_3, &extensions, None)
            .expect("Cannot create Vulkan instance.")
    };

    let event_loop = EventLoop::new();
    let surface = WindowBuilder::new()
        .with_title("ErupTrace")
        .build_vk_surface(&event_loop, instance.clone())
        .expect("Cannot create surface.");

    let (physical_device, device, queues) = {
        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::none()
        };
        let (physical_device, queue_family) = PhysicalDevice::enumerate(&instance)
            .filter(|&physical_device| {
                physical_device
                    .supported_extensions()
                    .is_superset_of(&device_extensions)
            })
            .filter_map(|physical_device| {
                physical_device
                    .queue_families()
                    .find(|&queue_family| {
                        queue_family.supports_graphics()
                            && surface.is_supported(queue_family).unwrap_or(false)
                    })
                    .map(|queue_family| (physical_device, queue_family))
            })
            .min_by_key(|(physical_device, _)| {
                use vulkano::device::physical::PhysicalDeviceType::*;
                match physical_device.properties().device_type {
                    DiscreteGpu => 0,
                    IntegratedGpu => 1,
                    VirtualGpu => 2,
                    Cpu => 3,
                    Other => 4,
                }
            })
            .expect("Cannot choose physical device and/or queue family.");
        let (device, queues) = {
            Device::new(
                physical_device,
                &Features::none(),
                &physical_device
                    .required_extensions()
                    .union(&device_extensions),
                [(queue_family, 0.5)].iter().cloned(),
            )
            .expect("Cannot create a device.")
        };
        let queues: Vec<Arc<Queue>> = queues.collect();
        (physical_device, device, queues)
    };

    let (mut swapchain, swapchain_images) = {
        let capabilities = surface
            .capabilities(physical_device)
            .expect("Cannot get physical device capabilities.");
        let composite_alpha = capabilities
            .supported_composite_alpha
            .iter()
            .next()
            .expect("Cannot get composite alpha support.");
        let &(format, _) = capabilities
            .supported_formats
            .get(0)
            .expect("Cannot get swapchain image format.");
        let dimensions: [u32; 2] = surface.window().inner_size().into();
        Swapchain::start(device.clone(), surface.clone())
            .num_images(capabilities.min_image_count)
            .format(format)
            .dimensions(dimensions)
            .usage(ImageUsage::color_attachment())
            .sharing_mode(&queues[0])
            .composite_alpha(composite_alpha)
            .build()
            .expect("Cannot create swapchain.")
    };

    let render_pass = vulkano::single_pass_renderpass!(
        device.clone(),
        attachments: {
            color: {
                load: Clear,
                store: Store,
                format: swapchain.format(),
                samples: 1,
            }
        },
        pass: {
            color: [color],
            depth_stencil: {}
        }
    )
    .expect("Cannot create render pass object.");

    let mut viewport = Viewport {
        origin: [0.0, 0.0],
        dimensions: [0.0, 0.0],
        depth_range: 0.0..1.0,
    };

    let mut framebuffers =
        update_viewport_and_create_framebuffers(&swapchain_images, &render_pass, &mut viewport);
    let mut recreate_swapchain = false;
    let mut prev_frame_end = Some(vulkano::sync::now(device.clone()).boxed());

    let camera_buf = {
        let camera_uniform: CameraUniform = camera.into();
        camera_uniform.to_buffer(device.clone())
    };
    let (
        scene_buffers,
        textures_future,
        normal_maps_future,
        materials_future,
        shapes_future,
        mesh_metas_future,
        mesh_data_future,
    ) = make_scene_buffers(queues[0].clone(), scene);

    let (render_surface, render_surface_vb_future) = RenderSurface::new(
        queues[0].clone(),
        render_pass.clone(),
        camera_buf.clone(),
        scene_buffers,
    );

    vulkano::sync::now(device.clone())
        .join(textures_future)
        .join(normal_maps_future)
        .join(materials_future)
        .join(shapes_future)
        .join(mesh_metas_future)
        .join(mesh_data_future)
        .join(render_surface_vb_future)
        .then_signal_fence_and_flush()
        .expect("Cannot flush.")
        .wait(None)
        .expect("Cannot wait.");

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {
                recreate_swapchain = true;
            }
            Event::RedrawEventsCleared => {
                prev_frame_end.as_mut().unwrap().cleanup_finished();

                if recreate_swapchain {
                    let dimensions: [u32; 2] = surface.window().inner_size().into();
                    match swapchain.recreate().dimensions(dimensions).build() {
                        Err(SwapchainCreationError::UnsupportedDimensions) => return,
                        Err(e) => panic!("Cannot recreate swapchain: {e:?}"),
                        Ok((new_swapchain, new_images)) => {
                            swapchain = new_swapchain;
                            framebuffers = update_viewport_and_create_framebuffers(
                                &new_images,
                                &render_pass,
                                &mut viewport,
                            );
                            recreate_swapchain = false;

                            match camera_buf.write() {
                                Ok(mut camera_lock) => {
                                    camera.img_size = dimensions;
                                    *camera_lock = camera.into();
                                }
                                Err(_) => eprintln!("Cannot get a write lock on camera buffer."),
                            }
                        }
                    };
                }

                match vulkano::swapchain::acquire_next_image(swapchain.clone(), None) {
                    Err(AcquireError::OutOfDate) => {
                        recreate_swapchain = true;
                    }
                    Err(e) => panic!("Cannot acquire the next image: {e:?}"),
                    Ok((image_num, suboptiomal, acquire_fut)) => {
                        if suboptiomal {
                            recreate_swapchain = true;
                        }

                        let cb = {
                            let clear_values = vec![[0.0, 0.0, 0.0, 1.0].into()];
                            let mut cb_builder = AutoCommandBufferBuilder::primary(
                                device.clone(),
                                queues[0].family(),
                                CommandBufferUsage::OneTimeSubmit,
                            )
                            .expect("Cannot start building command buffer.");
                            cb_builder
                                .begin_render_pass(
                                    framebuffers[image_num].clone(),
                                    SubpassContents::Inline,
                                    clear_values,
                                )
                                .expect("Cannot begin render pass.")
                                .set_viewport(0, [viewport.clone()]);
                            render_surface.draw(&mut cb_builder);
                            cb_builder
                                .end_render_pass()
                                .expect("Cannot end render pass.");
                            cb_builder.build().expect("Cannot build command buffer.")
                        };

                        let f = prev_frame_end
                            .take()
                            .expect("There's no value in prev_frame_end.")
                            .join(acquire_fut)
                            .then_execute(queues[0].clone(), cb)
                            .expect("Cannot execute commands.")
                            .then_swapchain_present(queues[0].clone(), swapchain.clone(), image_num)
                            .then_signal_fence_and_flush();
                        prev_frame_end.replace(match f {
                            Ok(future) => future.boxed(),
                            Err(e) => {
                                if e == FlushError::OutOfDate {
                                    recreate_swapchain = true;
                                } else {
                                    eprintln!("Failed to flush: {e:?}");
                                }
                                vulkano::sync::now(device.clone()).boxed()
                            }
                        });
                    }
                }
            }
            _ => {}
        }
    })
}

fn update_viewport_and_create_framebuffers(
    images: &[Arc<SwapchainImage<Window>>],
    render_pass: &Arc<RenderPass>,
    viewport: &mut Viewport,
) -> Vec<Arc<Framebuffer>> {
    let [width, height] = images[0].dimensions().width_height();
    viewport.dimensions = [width as f32, height as f32];
    images
        .iter()
        .map(|image| {
            let view = ImageView::new(image.clone()).expect("Cannot create image view.");
            Framebuffer::start(render_pass.clone())
                .add(view)
                .expect("Cannot add image view to framebuffer.")
                .build()
                .expect("Cannot create framebuffer.")
        })
        .collect()
}
