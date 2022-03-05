use eruptrace_pure::run_app;
use eruptrace_scene::{camera::Camera, Scene};
use nalgebra_glm as glm;

fn main() {
    let camera = Camera {
        position: glm::vec3(1.5, 1.5, 1.5),
        look_at: glm::vec3(0.5, 0.5, 0.5),
        up: glm::vec3(0.0, 1.0, 0.0),
        vertical_fov: 90.0,
        img_size: [0, 0],
        samples: 50,
        max_reflections: 10,
    };
    let scene = Scene::load("example_scenes/cube").unwrap();
    run_app(camera, scene);
}
