use crate::ctx;
use futures::FutureExt;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct CachingFetcher<CTXEXT, FENS> {
    pub inner: Arc<FENS>,
    _marker: std::marker::PhantomData<CTXEXT>,
}

impl<CTXEXT, FENS> CachingFetcher<CTXEXT, FENS> {
    pub fn new(inner: Arc<FENS>) -> Self {
        Self {
            inner,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<CTXEXT, FENS> CachingFetcher<CTXEXT, FENS>
where
    CTXEXT: Send + Sync + 'static,
    FENS: super::Fetcher<CTXEXT> + Send + Sync + 'static,
{
    pub async fn fetch(
        &self,
        ctx: ctx::Context<CTXEXT>,
        id: &str,
    ) -> Result<
        Option<(objectiveai::ensemble::Ensemble, u64)>,
        objectiveai::error::ResponseError,
    > {
        // Clone the shared future while holding the lock, then release the lock before awaiting.
        // This prevents deadlocks when multiple concurrent fetches hash to the same DashMap shard.
        let shared = ctx
            .ensemble_cache
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
