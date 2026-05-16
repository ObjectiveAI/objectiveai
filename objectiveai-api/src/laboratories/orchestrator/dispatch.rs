//! Runtime-dispatched orchestrator wrapper.
//!
//! The api server's `Client::ORCH` type parameter is fixed at compile
//! time, but integration tests want to swap in the mock orchestrator
//! at startup so the spawned api server doesn't try to talk to a real
//! Docker daemon. This enum wraps the bollard / mock / unimplemented
//! orchestrators and dispatches at runtime; `run.rs` picks one based
//! on the `LABORATORY_USE_MOCK_ORCHESTRATOR` env var.
//!
//! Production builds (the default `orchestrator-bollard` feature)
//! always pick the bollard variant unless the env var is set; builds
//! without the feature pick the unimplemented variant.

use crate::ctx;

pub enum DispatchedOrchestrator {
    #[cfg(feature = "orchestrator-bollard")]
    Bollard(super::bollard::Orchestrator),
    Mock(super::mock::Orchestrator),
    Unimplemented(super::unimplemented::Orchestrator),
}

#[derive(Debug)]
pub enum DispatchedOrchestratorError {
    #[cfg(feature = "orchestrator-bollard")]
    Bollard(super::bollard::Error),
    Mock(super::mock::Error),
    Unimplemented(super::unimplemented::Error),
}

impl std::fmt::Display for DispatchedOrchestratorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(feature = "orchestrator-bollard")]
            Self::Bollard(e) => std::fmt::Display::fmt(e, f),
            Self::Mock(e) => std::fmt::Display::fmt(e, f),
            Self::Unimplemented(e) => std::fmt::Display::fmt(e, f),
        }
    }
}

impl std::error::Error for DispatchedOrchestratorError {}

impl objectiveai_sdk::error::StatusError for DispatchedOrchestratorError {
    fn status(&self) -> u16 {
        match self {
            #[cfg(feature = "orchestrator-bollard")]
            Self::Bollard(e) => e.status(),
            Self::Mock(e) => e.status(),
            Self::Unimplemented(e) => e.status(),
        }
    }

    fn message(&self) -> Option<serde_json::Value> {
        match self {
            #[cfg(feature = "orchestrator-bollard")]
            Self::Bollard(e) => e.message(),
            Self::Mock(e) => e.message(),
            Self::Unimplemented(e) => e.message(),
        }
    }
}

impl<CTXEXT: Send + Sync + 'static> super::Orchestrator<CTXEXT> for DispatchedOrchestrator {
    type Error = DispatchedOrchestratorError;

    fn spawn_containers(
        &self,
        ctx: &ctx::Context<CTXEXT, impl ctx::persistent_cache::PersistentCacheClient>,
        image: &str,
        num_builders: usize,
        execution_id: &str,
        binaries: &[(&str, &[u8])],
        env: &[(&str, &str)],
    ) -> impl std::future::Future<Output = Result<Vec<String>, Self::Error>> + Send {
        async move {
            match self {
                #[cfg(feature = "orchestrator-bollard")]
                Self::Bollard(o) => o
                    .spawn_containers(ctx, image, num_builders, execution_id, binaries, env)
                    .await
                    .map_err(DispatchedOrchestratorError::Bollard),
                Self::Mock(o) => o
                    .spawn_containers(ctx, image, num_builders, execution_id, binaries, env)
                    .await
                    .map_err(DispatchedOrchestratorError::Mock),
                Self::Unimplemented(o) => o
                    .spawn_containers(ctx, image, num_builders, execution_id, binaries, env)
                    .await
                    .map_err(DispatchedOrchestratorError::Unimplemented),
            }
        }
    }

    fn cleanup_containers(
        &self,
        ctx: &ctx::Context<CTXEXT, impl ctx::persistent_cache::PersistentCacheClient>,
        execution_id: &str,
        num_builders: usize,
    ) -> impl std::future::Future<Output = ()> + Send {
        async move {
            match self {
                #[cfg(feature = "orchestrator-bollard")]
                Self::Bollard(o) => o.cleanup_containers(ctx, execution_id, num_builders).await,
                Self::Mock(o) => o.cleanup_containers(ctx, execution_id, num_builders).await,
                Self::Unimplemented(o) => o.cleanup_containers(ctx, execution_id, num_builders).await,
            }
        }
    }
}
