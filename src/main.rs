use eruptrace_scene::Scene;
use std::path::PathBuf;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use eruptrace_rs::App;
use erupt::vk;


struct EruptraceArgs {
    scene_path: PathBuf,
}

impl EruptraceArgs {
    pub fn parse_args() -> Result<Self, pico_args::Error> {
        let mut pargs = pico_args::Arguments::from_env();

        let args = Self {
            scene_path: pargs.free_from_str()?,
        };

        Ok(args)
    }
}

fn main() {
    match EruptraceArgs::parse_args() {
        Ok(args) => {
            let (camera, scene) = Scene::load(args.scene_path).unwrap();

            let event_loop = EventLoop::new();
            let window = WindowBuilder::new()
                .with_title("ErupTrace")
                .build(&event_loop)
                .expect("Cannot create window");
            let mut app = App::new(&window, camera, scene).unwrap();

            event_loop.run(move |event, _, control_flow| {
                *control_flow = ControlFlow::Poll;
                match event {
                    Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                        *control_flow = ControlFlow::Exit;
                    }
                    Event::WindowEvent { event: WindowEvent::Resized(size), .. } => {
                        let [width, height]: [u32; 2] = size.into();
                        app.resize(vk::Extent2D { width, height })
                    }
                    Event::MainEventsCleared => {
                        app.render();
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
