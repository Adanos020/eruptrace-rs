#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Camera {
    position: [f32; 4],
    horizontal: [f32; 4],
    vertical: [f32; 4],
    bottom_left: [f32; 4],
    img_size: [f32; 2],
    img_size_inv: [f32; 2],
}

impl Camera {
    pub fn new(position: [f32; 3], img_size: [u32; 2]) -> Self {
        let img_size = [img_size[0] as f32, img_size[1] as f32];
        let aspect = img_size[0] / img_size[1];
        let viewport_height = 2.0;
        let viewport_width = aspect * viewport_height;
        let focal_length = 1.0;
        Self {
            position: [position[0], position[1], position[2], 0.0],
            horizontal: [viewport_width, 0.0, 0.0, 0.0],
            vertical: [0.0, viewport_height, 0.0, 0.0],
            bottom_left: [
                position[0] - (viewport_width * 0.5),
                position[1] - (viewport_height * 0.5),
                position[2] - focal_length,
                0.0,
            ],
            img_size,
            img_size_inv: [1.0 / img_size[0], 1.0 / img_size[1]],
        }
    }

    #[must_use]
    pub fn position(&self) -> [f32; 3] {
        [self.position[0], self.position[1], self.position[2]]
    }

    pub fn set_position(&mut self, position: [f32; 3]) {
        let aspect = self.img_size[0] / self.img_size[1];
        let viewport_height = 2.0;
        let viewport_width = aspect * viewport_height;
        let focal_length = 1.0;
        self.position = [position[0], position[1], position[2], 0.0];
        self.bottom_left = [
            position[0] - (viewport_width * 0.5),
            position[1] - (viewport_height * 0.5),
            position[2] - focal_length,
            0.0,
        ];
    }

    pub fn set_img_size(&mut self, img_size: [u32; 2]) {
        self.img_size = [img_size[0] as f32, img_size[1] as f32];
        let aspect = self.img_size[0] / self.img_size[1];
        let viewport_height = 2.0;
        let viewport_width = aspect * viewport_height;
        self.img_size_inv = [1.0 / self.img_size[0], 1.0 / self.img_size[1]];
        self.horizontal = [viewport_width, 0.0, 0.0, 0.0];
        self.vertical = [0.0, viewport_height, 0.0, 0.0];
        self.bottom_left[0] = self.position[0] - (viewport_width * 0.5);
        self.bottom_left[1] = self.position[1] - (viewport_height * 0.5);
    }
}
