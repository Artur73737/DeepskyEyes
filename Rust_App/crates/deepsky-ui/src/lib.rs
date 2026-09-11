//! deepsky-ui — main crate UI, GPUI isolato (README §22-23, §64-68).
//! Il resto dell'app non dipende mai da tipi GPUI: vedi gpui_isolation.rs.

pub mod app_state;
pub mod theme;
pub mod gpui_isolation;
pub mod pages;
pub mod components;
pub mod widgets;
pub mod bridge;
#[cfg(feature = "desktop")]
mod desktop;
#[cfg(feature = "desktop")]
mod chrome;
pub use bridge::*;
#[cfg(feature = "desktop")]
pub use desktop::run;
