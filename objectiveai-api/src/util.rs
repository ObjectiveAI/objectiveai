//! Utility types for streaming and choice indexing.

use dashmap::DashMap;
use futures::Stream;
use std::sync::atomic::AtomicU64;

/// Assigns sequential indices to concurrent streams in first-come-first-served order.
///
/// When multiple concurrent streams need unique indices, this struct ensures
/// each stream gets the next available index. The first stream to request an
/// index for a given native key gets index 0 (or `initial`), the next gets 1, etc.
///
/// Thread-safe: uses atomic operations and concurrent hash map.
pub struct ChoiceIndexer {
    /// Counter for the next index to assign.
    counter: AtomicU64,
    /// Map from native keys to their assigned indices.
    indices: DashMap<usize, u64>,
}

impl ChoiceIndexer {
    /// Creates a new choice indexer starting from the given initial value.
    pub fn new(initial: u64) -> Self {
        Self {
            counter: AtomicU64::new(initial),
            indices: DashMap::new(),
        }
    }

    /// Gets the index for a native key, assigning the next available index if new.
    ///
    /// First-come-first-served: the first caller for a given native key gets
    /// the current counter value, then the counter increments for the next caller.
    pub fn get(&self, native_index: usize) -> u64 {
        *self.indices.entry(native_index).or_insert_with(|| {
            self.counter
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        })
    }
}

/// A stream that yields exactly one item, then completes.
///
/// Useful for wrapping a single value in a stream interface,
/// particularly for error handling in streaming contexts.
pub struct StreamOnce<T>(Option<T>);

impl<T> StreamOnce<T> {
    /// Creates a new single-item stream containing the given item.
    pub fn new(item: T) -> Self {
        Self(Some(item))
    }
}

impl<T> Stream for StreamOnce<T>
where
    T: Unpin,
{
    type Item = T;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        std::task::Poll::Ready(self.as_mut().get_mut().0.take())
    }
}
