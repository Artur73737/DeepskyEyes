//! Focus: AF/AF-lock/manuale/distanza (README §16-17).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FocusControl {
    pub distance_diopters: f32,
    pub locked: bool,
}
