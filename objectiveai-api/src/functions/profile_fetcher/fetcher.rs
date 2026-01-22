use crate::ctx;

#[async_trait::async_trait]
pub trait Fetcher<CTXEXT> {
    async fn fetch(
        &self,
        ctx: ctx::Context<CTXEXT>,
        owner: &str,
        repository: &str,
        commit: Option<&str>,
    ) -> Result<
        Option<objectiveai::functions::profiles::response::GetProfile>,
        objectiveai::error::ResponseError,
    >;
}
