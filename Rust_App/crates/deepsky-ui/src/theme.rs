//! Dark astrophotography theme: near-black surfaces, star-white text.
//! Values are display-only constants; no camera or storage logic lives here.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub dark: bool,
}

impl Theme {
    pub fn dark() -> Self {
        Self { dark: true }
    }
    /// Background #05070d, surface #0b1220, text #e8eef7, accent amber #ffb454.
    pub fn background_rgb(&self) -> (u8, u8, u8) {
        if self.dark { (5, 7, 13) } else { (232, 238, 247) }
    }
    pub fn text_rgb(&self) -> (u8, u8, u8) {
        if self.dark { (232, 238, 247) } else { (5, 7, 13) }
    }
    pub fn accent_rgb(&self) -> (u8, u8, u8) {
        (255, 180, 84)
    }
}
