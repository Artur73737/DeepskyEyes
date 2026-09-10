//! Mock Android: risponde a discover/capture/transfer.
#[derive(Default)]
pub struct MockAndroid {
    pub connected: bool,
}
