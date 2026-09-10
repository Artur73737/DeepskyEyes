//! Ring buffer limitato: RAW lossless, preview lossy.
pub struct RingBuffer<T> {
    buf: Vec<Option<T>>,
    head: usize,
    len: usize,
}

impl<T> RingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        let mut buf = Vec::with_capacity(capacity);
        buf.resize_with(capacity, || None);
        Self { buf, head: 0, len: 0 }
    }
    /// Push con overwrite del più vecchio (policy preview). Per RAW usare controllo pieno prima.
    pub fn push_overwrite(&mut self, item: T) {
        let cap = self.buf.len();
        if cap == 0 {
            return;
        }
        self.buf[self.head] = Some(item);
        self.head = (self.head + 1) % cap;
        if self.len < cap {
            self.len += 1;
        }
    }
    pub fn is_full(&self) -> bool {
        self.len == self.buf.len()
    }
    pub fn len(&self) -> usize {
        self.len
    }
}
