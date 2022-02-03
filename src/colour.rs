pub struct Colour {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl Colour {
    pub const BLACK: Colour = Self::new(0.0, 0.0, 0.0);
    pub const WHITE: Colour = Self::new(1.0, 1.0, 1.0);
    pub const RED: Colour = Self::new(1.0, 0.0, 0.0);
    pub const GREEN: Colour = Self::new(0.0, 1.0, 0.0);
    pub const BLUE: Colour = Self::new(0.0, 0.0, 1.0);

    pub const fn new(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b }
    }
}
