use eruptrace_rs::{App, EruptraceArgs};
use winit::event_loop::{ControlFlow, EventLoop};

fn main() {
    match EruptraceArgs::parse_args() {
        Ok(args) => {
            let event_loop = EventLoop::new().unwrap();
            event_loop.set_control_flow(ControlFlow::Poll);
            let mut app = App::new(args);
            if let Err(e) = event_loop.run_app(&mut app) {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}
