use eruptrace_pure::run_app;
use eruptrace_scene::{camera::Camera, Scene};
use nalgebra_glm as glm;
use std::path::PathBuf;

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
            let camera = Camera {
                position: glm::vec3(1.5, 1.5, 1.5),
                look_at: glm::vec3(0.5, 0.5, 0.5),
                up: glm::vec3(0.0, 1.0, 0.0),
                vertical_fov: 90.0,
                img_size: [0, 0],
                samples: 50,
                max_reflections: 10,
            };
            let scene = Scene::load(args.scene_path).unwrap();
            run_app(camera, scene);
        }
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}
