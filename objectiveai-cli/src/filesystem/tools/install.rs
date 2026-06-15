//! Tool GitHub-install — the tool side of the shared install engine
//! ([`crate::filesystem::install`]). Tools are simpler than plugins: a
//! `cli_zip` bundle plus the manifest, no viewer. These methods are
//! thin wrappers driving the generic engine with `tools::Manifest`.

use super::super::Client;
use super::Manifest;
use crate::filesystem::install::{
    ExtraAsset, InstallKind, InstallManifest, platform_cli_zip,
    validate_install_inputs,
};

impl Client {
    /// Install a tool from a GitHub repository: fetch `objectiveai.json`,
    /// download the current platform's `cli_zip`, extract it into the
    /// tool's `cli/` dir, and persist the manifest verbatim. Thin
    /// wrapper over [`Client::install_from_github_at`]. The `bool` is
    /// always `true` on success (retained for symmetry with plugins).
    pub async fn install_tool(
        &self,
        owner: &str,
        repository: &str,
        commit_sha: Option<&str>,
        headers: Option<&indexmap::IndexMap<String, String>>,
        upgrade: bool,
    ) -> Result<bool, super::super::Error> {
        self.install_from_github_at::<Manifest>(
            "https://raw.githubusercontent.com",
            "https://github.com",
            owner,
            repository,
            commit_sha,
            headers,
            upgrade,
        )
        .await
    }

    /// Fetch + parse `<owner>/<repo>/<ref>/objectiveai.json` as a
    /// tool manifest. Exposed so callers can inspect it before
    /// committing to an install (e.g. for whitelist checks).
    pub async fn fetch_tool_manifest(
        &self,
        owner: &str,
        repository: &str,
        commit_sha: Option<&str>,
        headers: Option<&indexmap::IndexMap<String, String>>,
    ) -> Result<Manifest, super::super::Error> {
        self.fetch_manifest_at::<Manifest>(
            "https://raw.githubusercontent.com",
            owner,
            repository,
            commit_sha,
            headers,
        )
        .await
    }

    /// Given an already-parsed tool manifest, download its `cli_zip` and
    /// persist it.
    pub async fn install_tool_from_manifest(
        &self,
        owner: &str,
        repository: &str,
        manifest: &Manifest,
        headers: Option<&indexmap::IndexMap<String, String>>,
        upgrade: bool,
    ) -> Result<bool, super::super::Error> {
        // Public entry — callers may hand us a manifest with no fetch
        // ever happening, so validate inputs here (cheap + idempotent).
        validate_install_inputs(InstallKind::Tool, owner, repository, None)?;
        self.install_parsed_at::<Manifest>(
            "https://github.com",
            owner,
            repository,
            manifest,
            headers,
            upgrade,
        )
        .await
    }

    /// Test-only entry threading mock URL bases through the engine.
    #[cfg(test)]
    pub(super) async fn install_tool_at(
        &self,
        raw_base: &str,
        releases_base: &str,
        owner: &str,
        repository: &str,
        commit_sha: Option<&str>,
        headers: Option<&indexmap::IndexMap<String, String>>,
        upgrade: bool,
    ) -> Result<bool, super::super::Error> {
        self.install_from_github_at::<Manifest>(
            raw_base,
            releases_base,
            owner,
            repository,
            commit_sha,
            headers,
            upgrade,
        )
        .await
    }

    /// Test-only fetch-only entry, mirrors `install_tool_at`.
    #[cfg(test)]
    pub(super) async fn fetch_tool_manifest_at(
        &self,
        raw_base: &str,
        owner: &str,
        repository: &str,
        commit_sha: Option<&str>,
        headers: Option<&indexmap::IndexMap<String, String>>,
    ) -> Result<Manifest, super::super::Error> {
        self.fetch_manifest_at::<Manifest>(
            raw_base, owner, repository, commit_sha, headers,
        )
        .await
    }
}

/// Tools install via the shared engine: just the cli bundle, under the
/// `tools/` dir tree — no viewer, no reserved-name check.
impl InstallManifest for Manifest {
    const KIND: InstallKind = InstallKind::Tool;
    fn validate_manifest(&self) -> Result<(), &'static str> {
        self.validate()
    }
    fn version(&self) -> &str {
        &self.version
    }
    fn cli_zip_for_platform(&self) -> Option<&str> {
        platform_cli_zip(&self.cli_zip)
    }
    fn extra_assets(&self) -> Vec<ExtraAsset> {
        Vec::new()
    }
}
