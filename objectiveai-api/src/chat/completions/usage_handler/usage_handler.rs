use crate::ctx;
use std::sync::Arc;

#[async_trait::async_trait]
pub trait UsageHandler<CTXEXT> {
    async fn handle_usage(
        &self,
        ctx: ctx::Context<CTXEXT>,
        request: Option<Arc<objectiveai::chat::completions::request::ChatCompletionCreateParams>>,
        response: objectiveai::chat::completions::response::unary::ChatCompletion,
    );
}
