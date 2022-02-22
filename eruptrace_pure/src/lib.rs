pub mod camera;
pub mod render_surface;
pub mod scene;
pub mod shaders;

use crate::{
    camera::CameraUniform,
    render_surface::{RenderSurface, Vertex},
    scene::make_scene_buffers,
    shaders::rt_shaders,
};
use eruptrace_scene::{camera::Camera, example_scenes};
use nalgebra_glm as glm;
use std::sync::Arc;
use vulkano::{
    buffer::TypedBufferAccess,
    command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, SubpassContents},
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    device::{physical::PhysicalDevice, Device, DeviceExtensions, Features, Queue},
    image::{
        view::{ImageView, ImageViewType},
        ImageAccess, ImageUsage, SwapchainImage,
    },
    instance::Instance,
    pipeline::{
        graphics::{
            input_assembly::InputAssemblyState,
            vertex_input::BuffersDefinition,
            viewport::{Viewport, ViewportState},
        },
        GraphicsPipeline, Pipeline, PipelineBindPoint,
    },
    render_pass::{Framebuffer, RenderPass, Subpass},
    sampler::{Filter, Sampler, SamplerAddressMode},
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

pub fn run_app() {
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

    let render_surface = RenderSurface::new(device.clone(), queues[0].clone());

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

    let graphics_pipeline = {
        let vertex_shader =
            rt_shaders::load_vertex(device.clone()).expect("Cannot load vertex shader.");
        let fragment_shader =
            rt_shaders::load_fragment(device.clone()).expect("Cannot load fragment shader.");
        GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<Vertex>())
            .vertex_shader(
                vertex_shader
                    .entry_point("main")
                    .expect("Cannot bind vertex shader with entry point 'main'."),
                (),
            )
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .fragment_shader(
                fragment_shader
                    .entry_point("main")
                    .expect("Cannot bind fragment shader with entry point 'main'."),
                (),
            )
            .render_pass(
                Subpass::from(render_pass.clone(), 0)
                    .expect("Cannot create subpass for render pass."),
            )
            .build(device.clone())
            .expect("Cannot create graphics pipeline object.")
    };

    let mut viewport = Viewport {
        origin: [0.0, 0.0],
        dimensions: [0.0, 0.0],
        depth_range: 0.0..1.0,
    };

    let mut framebuffers =
        update_viewport_and_create_framebuffers(&swapchain_images, &render_pass, &mut viewport);

    let mut recreate_swapchain = false;
    let mut prev_frame_end = Some(vulkano::sync::now(device.clone()).boxed());

    let mut camera = Camera {
        position: glm::vec3(0.5, 1.0, 2.0),
        look_at: glm::vec3(0.0, 0.0, -1.0),
        up: glm::vec3(0.0, 1.0, 0.0),
        vertical_fov: 90.0,
        img_size: surface.window().inner_size().into(),
        samples: 100,
        max_reflections: 10,
    };
    let camera_buf = {
        let camera_uniform: CameraUniform = camera.into();
        camera_uniform.to_buffer(device.clone())
    };

    let scene = example_scenes::test_dark_scene();
    let (shapes_buf, materials_buf, textures_img) =
        make_scene_buffers(device.clone(), queues[0].clone(), scene);
    let textures_img_view = ImageView::start(textures_img)
        .ty(ImageViewType::Dim2dArray)
        .build()
        .expect("Cannot create textures image.");
    let textures_sampler = Sampler::start(device.clone())
        .filter(Filter::Linear)
        .address_mode(SamplerAddressMode::ClampToEdge)
        .build()
        .expect("Cannot build sampler for textures.");

    let uniform_descriptor_set = {
        let layout = graphics_pipeline
            .layout()
            .descriptor_set_layouts()
            .get(0)
            .expect("Cannot get the layout of descriptor set 0.");
        PersistentDescriptorSet::new(
            layout.clone(),
            [
                WriteDescriptorSet::buffer(0, camera_buf.clone()),
                WriteDescriptorSet::buffer(1, shapes_buf),
                WriteDescriptorSet::buffer(2, materials_buf),
                WriteDescriptorSet::image_view_sampler(3, textures_img_view, textures_sampler),
            ],
        )
        .expect("Cannot create descriptor set.")
    };

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
                                .set_viewport(0, [viewport.clone()])
                                .bind_pipeline_graphics(graphics_pipeline.clone())
                                .bind_descriptor_sets(
                                    PipelineBindPoint::Graphics,
                                    Arc::clone(graphics_pipeline.layout()),
                                    0,
                                    uniform_descriptor_set.clone(),
                                )
                                .bind_vertex_buffers(0, render_surface.vertex_buffer.clone())
                                .bind_index_buffer(render_surface.index_buffer.clone())
                                .draw_indexed(render_surface.index_buffer.len() as u32, 1, 0, 0, 0)
                                .expect("Cannot execute draw command.")
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
