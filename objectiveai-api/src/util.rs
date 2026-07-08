//! Utility types for streaming and choice indexing.

use futures::Stream;

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

/// Maps native slot keys to wire indices — DETERMINISTICALLY.
///
/// The wire `index` on completions / task chunks correlates chunks,
/// votes (`completion_index`), and final results across the stream.
/// It was previously assigned FIRST-COME-FIRST-SERVED across the
/// concurrently racing slot streams, which made the wire content
/// scheduler-dependent: identical seeded runs produced different
/// index attributions (and thus different snapshots/results) purely
/// by task arrival order. The index rule is now identity:
///
/// - `get(native)` = `initial + native` — the slot's own stable key
///   (`flat_swarm_index` for completions, `task_index` for task
///   chunks; retry slots use `flat_swarm_index + flat_swarm_len`,
///   still unique and stable).
///
/// Consumers correlate BY index (SDK `mergedList`, vote attribution,
/// the confidence maps), so the only observable change is that the
/// numbers are stable instead of arrival-ordered.
pub struct ChoiceIndexer {
    /// Offset added to every native key.
    initial: u64,
}

impl ChoiceIndexer {
    /// Creates a new choice indexer starting from the given initial value.
    pub fn new(initial: u64) -> Self {
        Self { initial }
    }

    /// The wire index for a native slot key: `initial + native_index`.
    pub fn get(&self, native_index: usize) -> u64 {
        self.initial + native_index as u64
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
