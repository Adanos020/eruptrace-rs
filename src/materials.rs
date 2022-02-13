#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ReflectiveMaterial {
    pub color: [f32; 4],
    pub fuzz: f32,
}
