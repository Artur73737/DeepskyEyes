//! Isolamento GPUI: presentation sostituibile.
//!
//! - GPUI 0.2.2 is pinned exactly in Cargo.toml behind the `desktop` feature.
//! - Windows: DirectX11 + DirectWrite; Linux: wayland/x11; macOS: Metal.
//! - The native adapter is `desktop`; `bridge` exposes only Rust data and channels.
//! - Questo crate espone trait astratti; l'adapter GPUI concreto vive qui dentro
//!   e non deve leakare nei crate core (protocol/transport/camera/...).
//! Vedi doc/11-gpui-desktop-ui.md

/// Backend identity retained for headless callers.
pub trait UiBackend {
    fn name(&self) -> &'static str;
}

/// Backend fittizio per skeleton senza GPUI.
pub struct NoopBackend;

#[cfg(feature = "desktop")]
pub struct GpuiBackend;

#[cfg(feature = "desktop")]
impl UiBackend for GpuiBackend {
    fn name(&self) -> &'static str {
        "gpui-0.2.2"
    }
}

impl UiBackend for NoopBackend {
    fn name(&self) -> &'static str {
        "noop"
    }
}
