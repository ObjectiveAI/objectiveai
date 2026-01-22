use crate::ctx;
use futures::Stream;
use std::sync::Arc;

#[async_trait::async_trait]
pub trait Client<CTXEXT> {
    async fn create_unary(
        self: Arc<Self>,
        ctx: ctx::Context<CTXEXT>,
        request: Arc<objectiveai::functions::profiles::computations::request::Request>,
    ) -> Result<
        objectiveai::functions::profiles::computations::response::unary::FunctionProfileComputation,
        objectiveai::error::ResponseError,
    >;

    async fn create_streaming(
        self: Arc<Self>,
        ctx: ctx::Context<CTXEXT>,
        request: Arc<objectiveai::functions::profiles::computations::request::Request>,
    ) -> Result<
        impl Stream<Item = Result<
            objectiveai::functions::profiles::computations::response::streaming::FunctionProfileComputationChunk,
            objectiveai::error::ResponseError,
        >>
            + Send
            + 'static,
        objectiveai::error::ResponseError,
    >;
}
