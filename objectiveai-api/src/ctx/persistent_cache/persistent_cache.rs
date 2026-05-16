//! Persistent cache trait for key-value storage.

use objectiveai_sdk::error::ResponseError;

/// A persistent cache client for simple key-value storage.
///
/// Implementations may back this with Redis, DynamoDB, filesystem, etc.
pub trait PersistentCacheClient: Send + Sync + std::fmt::Debug + 'static {
    /// Gets a value by namespace and key. Returns `Ok(None)` if the key does not exist.
    fn get(&self, namespace: &'static str, key: &str) -> impl std::future::Future<Output = Result<Option<String>, ResponseError>> + Send;

    /// Sets a value by namespace and key. `permanent` indicates the value will never
    /// change for this key (e.g. content-addressed by commit SHA).
    fn set(&self, namespace: &'static str, key: &str, value: &str, permanent: bool) -> impl std::future::Future<Output = Result<(), ResponseError>> + Send;
}
