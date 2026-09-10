//! Isolamento GPUI: presentation sostituibile.
//!
//! - GPUI pre-1.0 -> pinnare versione esatta in Cargo.toml quando si attiva.
//! - Windows: DirectX11 + DirectWrite; Linux: wayland/x11; macOS: Metal.
//! - Questo crate espone trait astratti; l'adapter GPUI concreto vive qui dentro
//!   e non deve leakare nei crate core (protocol/transport/camera/...).
//! Vedi doc/11-gpui-desktop-ui.md

/// Adapter astratto che la UI reale (GPUI) implementerà.
pub trait UiBackend {
    fn name(&self) -> &'static str;
}

/// Backend fittizio per skeleton senza GPUI.
pub struct NoopBackend;

impl UiBackend for NoopBackend {
    fn name(&self) -> &'static str {
        "noop"
    }
}
