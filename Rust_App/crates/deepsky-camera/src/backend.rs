//! Backend astratto: PixelCameraBackend oggi, DSLR/astro/simulator domani (README §88).
pub trait CameraBackend {
    fn id(&self) -> &str;
}
