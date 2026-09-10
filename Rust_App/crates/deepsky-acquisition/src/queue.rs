//! Coda acquisizione limitata — mai crescita infinita.
use std::collections::VecDeque;

pub struct BoundedQueue<T> {
    inner: VecDeque<T>,
    capacity: usize,
}

impl<T> BoundedQueue<T> {
    pub fn new(capacity: usize) -> Self {
        Self { inner: VecDeque::new(), capacity }
    }
    pub fn push(&mut self, item: T) -> Result<(), T> {
        if self.inner.len() >= self.capacity {
            return Err(item);
        }
        self.inner.push_back(item);
        Ok(())
    }
    pub fn pop(&mut self) -> Option<T> {
        self.inner.pop_front()
    }
    pub fn len(&self) -> usize {
        self.inner.len()
    }
}
