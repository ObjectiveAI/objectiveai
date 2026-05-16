//! Default (no-op) persistent cache client.

use objectiveai_sdk::error::ResponseError;

/// A no-op persistent cache client that never stores or retrieves anything.
#[derive(Debug)]
pub struct DefaultPersistentCacheClient;

impl super::PersistentCacheClient for DefaultPersistentCacheClient {
    fn get(&self, _namespace: &'static str, _key: &str) -> impl std::future::Future<Output = Result<Option<String>, ResponseError>> + Send {
        std::future::ready(Ok(None))
    }

    fn set(&self, _namespace: &'static str, _key: &str, _value: &str, _permanent: bool) -> impl std::future::Future<Output = Result<(), ResponseError>> + Send {
        std::future::ready(Ok(()))
    }
}
