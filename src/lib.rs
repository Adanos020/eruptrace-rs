use crate::{
    camera::Camera,
    render_surface::{RenderSurface, Vertex},
    scene::Scene,
    shaders::rt_shaders,
};
use std::sync::Arc;
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer, ImmutableBuffer, TypedBufferAccess},
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
    swapchain::{AcquireError, Surface, Swapchain, SwapchainCreationError},
    sync::{FlushError, GpuFuture},
    Version,
};
use vulkano_win::VkSurfaceBuild;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

mod camera;
mod materials;
mod primitives;
mod render_surface;
mod scene;
mod shaders;

pub fn run_app() {
    let instance = {
        let extensions = vulkano_win::required_extensions();
        Instance::new(None, Version::V1_3, &extensions, None)
            .expect("Cannot create Vulkan instance.")
    };

    let event_loop = EventLoop::new();
    let surface = WindowBuilder::new()
        .with_title("ErupTrace")
        .build_vk_surface(&event_loop, Arc::clone(&instance))
        .expect("Cannot create surface.");

    let (physical_device, device, queues) = get_device_and_queues(&instance, &surface);
    let (mut swapchain, swapchain_images) =
        create_swapchain(&surface, physical_device, &device, &queues);
    let render_surface = RenderSurface::new(&device, &queues[0]);
    let render_pass = create_render_pass(&device, &swapchain);
    let graphics_pipeline = create_graphics_pipeline(&device, &render_pass);

    let mut viewport = Viewport {
        origin: [0.0, 0.0],
        dimensions: [0.0, 0.0],
        depth_range: 0.0..1.0,
    };

    let mut framebuffers =
        update_viewport_and_create_framebuffers(&swapchain_images, &render_pass, &mut viewport);

    let mut recreate_swapchain = false;
    let mut prev_frame_end = Some(vulkano::sync::now(Arc::clone(&device)).boxed());

    let camera_buf = {
        let camera = Camera::new(
            [0.0, 0.0, 0.0],
            surface.window().inner_size().into(),
            30,
            20,
        );
        CpuAccessibleBuffer::from_data(
            Arc::clone(&device),
            BufferUsage::uniform_buffer(),
            false,
            camera,
        )
        .expect("Cannot create uniform buffer for camera.")
    };

    let scene = Scene::test_scene();

    let textures_img = scene.get_texture_data(device.clone(), queues[0].clone());
    let textures_img_view = ImageView::start(textures_img)
        .ty(ImageViewType::Dim2dArray)
        // .format(Format::R8G8B8A8_UNORM)
        .build()
        .expect("Cannot create textures image.");
    let textures_sampler = Sampler::start(device.clone())
        .filter(Filter::Linear)
        .address_mode(SamplerAddressMode::ClampToEdge)
        .build()
        .expect("Cannot build sampler for textures.");

    let (shapes_buf, materials_buf) = {
        let (shapes_buf, shapes_fut) = ImmutableBuffer::from_iter(
            scene.get_shape_data().into_iter(),
            BufferUsage::storage_buffer(),
            Arc::clone(&queues[0]),
        )
        .expect("Cannot create buffer for shapes in scene.");
        let (materials_buf, materials_fut) = ImmutableBuffer::from_iter(
            scene.materials.into_iter(),
            BufferUsage::storage_buffer(),
            Arc::clone(&queues[0]),
        )
        .expect("Cannot create buffer for materials in scene.");
        let _ = vulkano::sync::now(Arc::clone(&device))
            .join(shapes_fut)
            .join(materials_fut)
            .then_signal_fence_and_flush()
            .expect("Cannot upload shapes data buffer.");
        (shapes_buf, materials_buf)
    };

    let uniform_descriptor_set = {
        let layout = graphics_pipeline
            .layout()
            .descriptor_set_layouts()
            .get(0)
            .expect("Cannot get the layout of descriptor set 0.");
        PersistentDescriptorSet::new(
            Arc::clone(layout),
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
                                Ok(mut camera_lock) => camera_lock.set_img_size(dimensions),
                                Err(_) => eprintln!("Cannot get a write lock on camera buffer."),
                            }
                        }
                    };
                }

                match vulkano::swapchain::acquire_next_image(Arc::clone(&swapchain), None) {
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
                                Arc::clone(&device),
                                queues[0].family(),
                                CommandBufferUsage::OneTimeSubmit,
                            )
                            .expect("Cannot start building command buffer.");
                            cb_builder
                                .begin_render_pass(
                                    Arc::clone(&framebuffers[image_num]),
                                    SubpassContents::Inline,
                                    clear_values,
                                )
                                .expect("Cannot begin render pass.")
                                .set_viewport(0, [viewport.clone()])
                                .bind_pipeline_graphics(Arc::clone(&graphics_pipeline))
                                .bind_descriptor_sets(
                                    PipelineBindPoint::Graphics,
                                    Arc::clone(graphics_pipeline.layout()),
                                    0,
                                    Arc::clone(&uniform_descriptor_set),
                                )
                                .bind_vertex_buffers(0, Arc::clone(&render_surface.vertex_buffer))
                                .bind_index_buffer(Arc::clone(&render_surface.index_buffer))
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
                            .then_execute(Arc::clone(&queues[0]), cb)
                            .expect("Cannot execute commands.")
                            .then_swapchain_present(
                                Arc::clone(&queues[0]),
                                Arc::clone(&swapchain),
                                image_num,
                            )
                            .then_signal_fence_and_flush();
                        prev_frame_end.replace(match f {
                            Ok(future) => future.boxed(),
                            Err(e) => {
                                if e == FlushError::OutOfDate {
                                    recreate_swapchain = true;
                                } else {
                                    eprintln!("Failed to flush: {e:?}");
                                }
                                vulkano::sync::now(Arc::clone(&device)).boxed()
                            }
                        });
                    }
                }
            }
            _ => {}
        }
    })
}

fn get_device_and_queues<'a>(
    instance: &'a Arc<Instance>,
    surface: &Arc<Surface<Window>>,
) -> (PhysicalDevice<'a>, Arc<Device>, Vec<Arc<Queue>>) {
    let device_extensions = DeviceExtensions {
        khr_swapchain: true,
        ..DeviceExtensions::none()
    };
    let (physical_device, queue_family) = PhysicalDevice::enumerate(instance)
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
    (physical_device, device, queues.collect())
}

fn create_swapchain(
    surface: &Arc<Surface<Window>>,
    physical_device: PhysicalDevice,
    device: &Arc<Device>,
    queues: &[Arc<Queue>],
) -> (Arc<Swapchain<Window>>, Vec<Arc<SwapchainImage<Window>>>) {
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
    Swapchain::start(Arc::clone(device), Arc::clone(surface))
        .num_images(capabilities.min_image_count)
        .format(format)
        .dimensions(dimensions)
        .usage(ImageUsage::color_attachment())
        .sharing_mode(&queues[0])
        .composite_alpha(composite_alpha)
        .build()
        .expect("Cannot create swapchain.")
}

fn create_render_pass(device: &Arc<Device>, swapchain: &Arc<Swapchain<Window>>) -> Arc<RenderPass> {
    vulkano::single_pass_renderpass!(
        Arc::clone(device),
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
    .expect("Cannot create render pass object.")
}

fn create_graphics_pipeline(
    device: &Arc<Device>,
    render_pass: &Arc<RenderPass>,
) -> Arc<GraphicsPipeline> {
    let vertex_shader =
        rt_shaders::load_vertex(Arc::clone(device)).expect("Cannot load vertex shader.");
    let fragment_shader =
        rt_shaders::load_fragment(Arc::clone(device)).expect("Cannot load fragment shader.");
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
            Subpass::from(Arc::clone(render_pass), 0)
                .expect("Cannot create subpass for render pass."),
        )
        .build(Arc::clone(device))
        .expect("Cannot create graphics pipeline object.")
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
            let view = ImageView::new(Arc::clone(image)).expect("Cannot create image view.");
            Framebuffer::start(Arc::clone(render_pass))
                .add(view)
                .expect("Cannot add image view to framebuffer.")
                .build()
                .expect("Cannot create framebuffer.")
        })
        .collect()
}
