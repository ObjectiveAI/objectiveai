//! Trait for listing Functions and getting usage statistics.

use crate::ctx;

/// Client for listing Functions and retrieving usage statistics.
#[async_trait::async_trait]
pub trait Client<CTXEXT> {
    /// Lists Functions.
    async fn list_functions(
        &self,
        ctx: ctx::Context<CTXEXT>,
    ) -> Result<
        objectiveai::functions::response::ListFunction,
        objectiveai::error::ResponseError,
    >;

    /// Retrieves usage statistics for a Function.
    async fn get_function_usage(
        &self,
        ctx: ctx::Context<CTXEXT>,
        owner: &str,
        repository: &str,
        commit: Option<&str>,
    ) -> Result<
        objectiveai::functions::response::UsageFunction,
        objectiveai::error::ResponseError,
    >;
}
