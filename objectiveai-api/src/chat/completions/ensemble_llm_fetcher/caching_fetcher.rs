use crate::ctx;
use futures::FutureExt;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct CachingFetcher<CTXEXT, FENSLLM> {
    pub inner: Arc<FENSLLM>,
    _marker: std::marker::PhantomData<CTXEXT>,
}

impl<CTXEXT, FENSLLM> CachingFetcher<CTXEXT, FENSLLM> {
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
