//! Utility types for streaming and choice indexing.

use dashmap::DashMap;
use futures::Stream;
use std::sync::atomic::AtomicU64;

/// Mint a response id of the form `{prefix}-{rand_b62}{created_b62}`,
/// or `{rand_b62}{created_b62}` when `prefix` is `None`.
///
/// `rand_b62` is a u64 random value base62-encoded and zero-padded
/// to 11 characters (the max width for any u64 in base62). The
/// random half is fixed-width so the split point between random
/// entropy and the timestamp is deterministic at offset 11 (after
/// any prefix + dash). `created_b62` is the unpadded base62 of the
/// Unix timestamp — it grows monotonically and just appends after
/// the random half.
pub fn response_id(prefix: Option<&str>, created: u64) -> String {
    let rand: u64 = rand::random();
    let rand_b62 = format!("{:0>11}", base62::encode(rand as u128));
    let ts_b62 = base62::encode(created as u128);
    match prefix {
        Some(p) => format!("{p}-{rand_b62}{ts_b62}"),
        None => format!("{rand_b62}{ts_b62}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_id_no_prefix_shape() {
        let id = response_id(None, 1_700_000_000);
        assert!(!id.contains('-'), "no-prefix id must not contain a dash, got {id:?}");
        assert!(id.len() > 11, "id should have content past the random half, got {id:?}");
        for c in id.chars() {
            assert!(c.is_ascii_alphanumeric(), "non-base62 char in {id:?}");
        }
    }

    #[test]
    fn response_id_with_prefix_shape() {
        let id = response_id(Some("vctcpl"), 1_700_000_000);
        let dash_pos = id.find('-').expect("prefixed id should have a dash");
        let (prefix, rest) = id.split_at(dash_pos);
        assert_eq!(prefix, "vctcpl");
        // rest starts with '-', then 11 char rand, then timestamp.
        let body = &rest[1..];
        assert!(!body.contains('-'), "body after the prefix dash must not contain a dash, got {id:?}");
        assert!(body.len() > 11, "body should have content past the random half, got {id:?}");
    }
}

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
