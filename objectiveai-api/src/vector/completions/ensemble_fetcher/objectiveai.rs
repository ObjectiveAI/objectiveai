use crate::ctx;
use objectiveai::error::StatusError;
use std::sync::Arc;

pub struct ObjectiveAiFetcher {
    pub client: Arc<objectiveai::HttpClient>,
}

impl ObjectiveAiFetcher {
    pub fn new(client: Arc<objectiveai::HttpClient>) -> Self {
        Self { client }
    }
}

#[async_trait::async_trait]
impl<CTXEXT> super::Fetcher<CTXEXT> for ObjectiveAiFetcher
where
    CTXEXT: Send + Sync + 'static,
{
    async fn fetch(
        &self,
        _ctx: ctx::Context<CTXEXT>,
        id: &str,
    ) -> Result<
        Option<(objectiveai::ensemble::Ensemble, u64)>,
        objectiveai::error::ResponseError,
    > {
        match objectiveai::ensemble::get_ensemble(&self.client, id).await {
            Ok(ensemble) => Ok(Some((ensemble.inner, ensemble.created))),
            Err(e) if e.status() == 404 => Ok(None),
            Err(e) => Err(objectiveai::error::ResponseError::from(&e)),
        }
    }
}
