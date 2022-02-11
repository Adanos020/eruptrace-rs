#[repr(C)]
#[derive(Copy, Clone)]
pub struct Sphere {
    pub color: [f32; 4],
    pub position: [f32; 3],
    pub radius: f32,
}
