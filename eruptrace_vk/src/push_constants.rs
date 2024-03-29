use bitflags::bitflags;
use nalgebra_glm as glm;

bitflags! {
    #[derive(Copy, Clone, Debug)]
    pub struct RtFlags: u32 {
        const USE_BIH = 1 << 0;
        const RENDER_NORMALS = 1 << 1;
        const RENDER_BIH = 1 << 2;
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RtPushConstants {
    pub n_triangles:    u32,
    pub flags:          RtFlags,
    pub draw_bih_level: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct GuiPushConstants {
    pub screen_size: glm::Vec2,
}
