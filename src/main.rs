use eruptrace_pure::run_app;
use eruptrace_scene::Scene;
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
            let (camera, scene) = Scene::load(args.scene_path).unwrap();
            run_app(camera, scene);
        }
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}
