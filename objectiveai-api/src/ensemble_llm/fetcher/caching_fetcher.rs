//! Caching wrapper for Ensemble LLM fetchers.

use crate::ctx;
use futures::FutureExt;
use std::sync::Arc;

/// Wraps an Ensemble LLM fetcher with per-request deduplication caching.
///
/// When multiple parts of a request need the same Ensemble LLM, this fetcher
/// ensures only one actual fetch is performed. Subsequent requests for the
/// same Ensemble LLM ID within the same request context share the result.
#[derive(Debug, Clone)]
pub struct CachingFetcher<CTXEXT, FENSLLM> {
    /// The underlying fetcher to delegate to on cache miss.
    pub inner: Arc<FENSLLM>,
    _marker: std::marker::PhantomData<CTXEXT>,
}

impl<CTXEXT, FENSLLM> CachingFetcher<CTXEXT, FENSLLM> {
    /// Creates a new caching fetcher wrapping the given inner fetcher.
    pub fn new(inner: Arc<FENSLLM>) -> Self {
        Self {
            inner,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<CTXEXT, FENSLLM> CachingFetcher<CTXEXT, FENSLLM>
where
    CTXEXT: Send + Sync + 'static,
    FENSLLM: super::Fetcher<CTXEXT> + Send + Sync + 'static,
{
    /// Spawns concurrent fetch tasks for multiple Ensemble LLM IDs.
    ///
    /// This allows pre-warming the cache when the set of required IDs is known
    /// ahead of time, reducing latency by parallelizing the fetches.
    pub fn spawn_fetches<'id>(
        &self,
        ctx: ctx::Context<CTXEXT>,
        ids: impl Iterator<Item = &'id str>,
    ) {
        for id in ids {
            ctx.ensemble_llm_cache
                .entry(id.to_owned())
                .or_insert_with(|| {
                    let (tx, rx) = tokio::sync::oneshot::channel();
                    let inner = self.inner.clone();
                    let id = id.to_owned();
                    let ctx = ctx.clone();
                    tokio::spawn(async move {
                        let result = inner.fetch(ctx, &id).await;
                        let _ = tx.send(result);
                    });
                    rx.shared()
                });
        }
    }

    /// Fetches an Ensemble LLM, using the request-scoped cache for deduplication.
    ///
    /// If another fetch for the same ID is already in progress within this
    /// request context, waits for and shares that result instead of fetching again.
    pub async fn fetch(
        &self,
        ctx: ctx::Context<CTXEXT>,
        id: &str,
    ) -> Result<
        Option<(objectiveai::ensemble_llm::EnsembleLlm, u64)>,
        objectiveai::error::ResponseError,
    > {
        // Clone the shared future while holding the lock, then release the lock before awaiting.
        // This prevents deadlocks when multiple concurrent fetches hash to the same DashMap shard.
        let shared = ctx
            .ensemble_llm_cache
            .entry(id.to_owned())
            .or_insert_with(|| {
                let (tx, rx) = tokio::sync::oneshot::channel();
                let inner = self.inner.clone();
                let id = id.to_owned();
                let ctx = ctx.clone();
                tokio::spawn(async move {
                    let result = inner.fetch(ctx, &id).await;
                    let _ = tx.send(result);
                });
                rx.shared()
            })
            .clone();
        // Lock is now released, safe to await
        shared.await.unwrap()
    }
}
