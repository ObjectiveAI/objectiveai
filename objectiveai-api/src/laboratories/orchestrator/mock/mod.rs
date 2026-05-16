use crate::ctx;

/// Mock orchestrator that returns fake MCP URLs without spawning any
/// containers. For use in tests where the mock upstream client ignores
/// MCP servers entirely.
pub struct Orchestrator;

/// Error type for the mock orchestrator. Never constructed.
#[derive(Debug)]
pub enum Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {}
    }
}

impl objectiveai_sdk::error::StatusError for Error {
    fn status(&self) -> u16 { match *self {} }
    fn message(&self) -> Option<serde_json::Value> { match *self {} }
}

impl<CTXEXT: Send + Sync + 'static> super::Orchestrator<CTXEXT> for Orchestrator {
    type Error = Error;

    fn spawn_containers(
        &self,
        _ctx: &ctx::Context<CTXEXT, impl ctx::persistent_cache::PersistentCacheClient>,
        _image: &str,
        num_builders: usize,
        execution_id: &str,
        _binaries: &[(&str, &[u8])],
        _env: &[(&str, &str)],
    ) -> impl std::future::Future<Output = Result<Vec<String>, Self::Error>> + Send {
        let urls: Vec<String> = (0..num_builders)
            .map(|_| "mock".to_string())
            .collect();
        async move { Ok(urls) }
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
