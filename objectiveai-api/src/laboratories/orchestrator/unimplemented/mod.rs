use crate::ctx;

/// Stub orchestrator that always returns an unimplemented error.
/// Used when no orchestrator backend (e.g. bollard, GCP) is enabled.
pub struct Orchestrator;

#[derive(Debug, thiserror::Error)]
#[error("laboratories orchestrator not implemented")]
pub struct Error;

impl objectiveai_sdk::error::StatusError for Error {
    fn status(&self) -> u16 {
        501
    }

    fn message(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "kind": "orchestrator",
            "error": {
                "kind": "unimplemented",
                "error": "laboratories orchestrator not implemented",
            }
        }))
    }
}

impl<CTXEXT: Send + Sync + 'static> super::Orchestrator<CTXEXT> for Orchestrator {
    type Error = Error;

    fn spawn_containers(
        &self,
        _ctx: &ctx::Context<CTXEXT, impl ctx::persistent_cache::PersistentCacheClient>,
        _image: &str,
        _num_builders: usize,
        _execution_id: &str,
        _binaries: &[(&str, &[u8])],
        _env: &[(&str, &str)],
    ) -> impl std::future::Future<Output = Result<Vec<String>, Self::Error>> + Send {
        async { Err(Error) }
    }

    fn cleanup_containers(
        &self,
        _ctx: &ctx::Context<CTXEXT, impl ctx::persistent_cache::PersistentCacheClient>,
        _execution_id: &str,
        _num_builders: usize,
    ) -> impl std::future::Future<Output = ()> + Send {
        async {}
    }
}
