//! Cache preview newest-wins.
#[derive(Default)]
pub struct PreviewCache {
    pub last: Option<super::frame::PreviewFrame>,
}
