use crate::ctx;

#[async_trait::async_trait]
pub trait Fetcher<CTXEXT> {
    async fn fetch(
        &self,
        ctx: ctx::Context<CTXEXT>,
        id: &str,
    ) -> Result<
        Option<(objectiveai::ensemble_llm::EnsembleLlm, u64)>,
        objectiveai::error::ResponseError,
    >;
}
