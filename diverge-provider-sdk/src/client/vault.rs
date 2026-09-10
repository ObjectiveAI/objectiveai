//! The vault a container keeps its secrets in, which lives with the
//! caller.

use std::future::Future;

use bytes::Bytes;

/// A key-value store with locks — the caller's side of
/// [`vault`](crate::shared::containers::vault).
///
/// Keys are the container's; WHICH vault they land in — whose, scoped
/// how — is the implementor's, and the wire carries no namespace for
/// the container to name. An `Err` is the text the wire's `Error`
/// frame carries, for a reader rather than a program.
///
/// # The lock
///
/// [`lock`](Self::lock) resolves when the lock is HELD — however long
/// that takes, since nothing times anything out — and its TTL runs
/// from the grant. The holder is the container, not any connection:
/// a lock on a key the container already holds refreshes the TTL and
/// resolves at once; expiry releases silently; [`unlock`](Self::unlock)
/// releases early, and an unlock from a container that does not hold
/// the key is an error. A TTL of `0` is refused. Which container is
/// asking is the implementor's to know from the scope it serves.
pub trait Vault: Send + Sync {
    /// The value under `key`: `Ok(None)` for a key not held. Empty is
    /// a value.
    fn get(&self, key: &str) -> impl Future<Output = Result<Option<Bytes>, String>> + Send;

    /// Set `key` to `value`, making it if absent.
    fn set(&self, key: &str, value: Bytes) -> impl Future<Output = Result<(), String>> + Send;

    /// Remove `key`. Removing a key not held is not an error.
    fn delete(&self, key: &str) -> impl Future<Output = Result<(), String>> + Send;

    /// Hold `key`'s lock for `ttl` seconds from the grant, resolving
    /// when held.
    fn lock(&self, key: &str, ttl: u32) -> impl Future<Output = Result<(), String>> + Send;

    /// Release `key`'s lock early.
    fn unlock(&self, key: &str) -> impl Future<Output = Result<(), String>> + Send;
}
