use bitflags::bitflags;

bitflags! {
    pub struct RtFlags: u32 {
        const USE_BIH = 1 << 0;
        const RENDER_NORMALS = 1 << 1;
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RtPushConstants {
    pub n_triangles: u32,
    pub flags:       RtFlags,
}
