//! Trait for fetching Profile definitions.

use crate::ctx;

/// Fetches Profile definitions from GitHub repositories.
///
/// Profiles are stored as `profile.json` at repository root and referenced
/// by owner/repository (optionally with commit SHA).
#[async_trait::async_trait]
pub trait Fetcher<CTXEXT> {
    /// Fetches a Profile by owner/repository/commit.
    ///
    /// Returns None if the Profile is not found.
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
