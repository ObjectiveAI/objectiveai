pub mod mock;
pub mod unimplemented;

#[cfg(feature = "orchestrator-bollard")]
pub mod bollard;

pub mod dispatch;

use crate::ctx;

/// Abstracts the container lifecycle for laboratory executions.
///
/// Implementations handle spawning builder environments (containers, VMs,
/// serverless instances, etc.), uploading and executing binaries inside them,
/// exposing MCP server URLs, and cleanup.
pub trait Orchestrator<CTXEXT>: Send + Sync + 'static {
    type Error: objectiveai_sdk::error::StatusError + Send + Sync + 'static;

    /// Spawn `num_builders` builder environments from the given image.
    ///
    /// Each binary in `binaries` is uploaded and executed inside every builder
    /// with the given `env` environment variables. Returns one MCP server URL
    /// per builder, in order.
    fn spawn_containers(
        &self,
        ctx: &ctx::Context<CTXEXT, impl ctx::persistent_cache::PersistentCacheClient>,
        image: &str,
        num_builders: usize,
        execution_id: &str,
        binaries: &[(&str, &[u8])],
        env: &[(&str, &str)],
    ) -> impl std::future::Future<Output = Result<Vec<String>, Self::Error>> + Send;

    /// Clean up all builder environments for the given execution.
    ///
    /// Implementations should be idempotent (safe to call multiple times).
    fn cleanup_containers(
        &self,
        ctx: &ctx::Context<CTXEXT, impl ctx::persistent_cache::PersistentCacheClient>,
        execution_id: &str,
        num_builders: usize,
    ) -> impl std::future::Future<Output = ()> + Send;
}
