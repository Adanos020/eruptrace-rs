use std::path::PathBuf;

use erupt::vk;
use eruptrace_rs::App;
use eruptrace_scene::Scene;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

struct EruptraceArgs {
    scene_path: PathBuf,
}

impl EruptraceArgs {
    pub fn parse_args() -> Result<Self, pico_args::Error> {
        let mut pargs = pico_args::Arguments::from_env();

        let args = Self { scene_path: pargs.free_from_str()? };

        Ok(args)
    }
}

fn main() {
    match EruptraceArgs::parse_args() {
        Ok(args) => {
            let (camera, scene) = Scene::load(args.scene_path).unwrap();

            let event_loop = EventLoop::new();
            let window = WindowBuilder::new().with_title("ErupTrace").build(&event_loop).expect("Cannot create window");
            let mut app = App::new(&window, camera, scene).unwrap();
            let mut egui_winit_state = egui_winit::State::new(4096, &window);
            let egui_context = egui::Context::default();

            event_loop.run(move |event, _, control_flow| {
                *control_flow = ControlFlow::Poll;
                let new_input = egui_winit_state.take_egui_input(&window);
                match event {
                    Event::WindowEvent { event, .. } => {
                        egui_winit_state.on_event(&egui_context, &event);
                        match event {
                            WindowEvent::CloseRequested => {
                                *control_flow = ControlFlow::Exit;
                            }
                            WindowEvent::Resized(size) => {
                                let [width, height]: [u32; 2] = size.into();
                                app.resize(vk::Extent2D { width, height })
                            }
                            _ => (),
                        }
                    }
                    Event::MainEventsCleared => {
                        let full_output = egui_context.run(new_input, |egui_context| app.gui(egui_context));
                        let clipped_meshes = egui_context.tessellate(full_output.shapes);
                        app.render(&full_output.textures_delta, clipped_meshes);
                        egui_winit_state.handle_platform_output(&window, &egui_context, full_output.platform_output);
                    }
                    _ => {}
                }
            })
        }
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}
