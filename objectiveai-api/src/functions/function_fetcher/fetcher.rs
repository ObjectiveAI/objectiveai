//! Trait for fetching Function definitions.

use crate::ctx;

/// Fetches Function definitions from GitHub repositories.
///
/// Functions are stored as `function.json` at repository root and referenced
/// by owner/repository (optionally with commit SHA).
#[async_trait::async_trait]
pub trait Fetcher<CTXEXT> {
    /// Fetches a Function by owner/repository/commit.
    ///
    /// Returns None if the Function is not found.
    async fn fetch(
        &self,
        ctx: ctx::Context<CTXEXT>,
        owner: &str,
        repository: &str,
        commit: Option<&str>,
    ) -> Result<
        Option<objectiveai::functions::response::GetFunction>,
        objectiveai::error::ResponseError,
    >;
}
