//! `plugins install github` — fetch the manifest, check the
//! whitelist, and install the plugin under
//! `~/.objectiveai/plugins/<owner>/<repository>/<version>/`. The
//! bare-naked contract surfaces the untrusted decision as the typed
//! `Error::NotWhitelisted { kind: "plugin", .. }` variant.
//!
//! The SDK `Request` does not expose `upgrade`; this leaf always
//! installs fresh and surfaces `Error::AlreadyInstalled` if a
//! manifest already exists.

use objectiveai_sdk::cli::command::plugins::install::github::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(_global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<Response, Error> {
    let manifest = scoped
        .filesystem
        .fetch_plugin_manifest(
            &request.owner,
            &request.repository,
            request.commit_sha.as_deref(),
            None,
        )
        .await?;

    let effective_sha = request.commit_sha.as_deref().unwrap_or("HEAD");
    let whitelist = crate::filesystem::install::default_whitelist();
    let allowed = crate::filesystem::install::check_plugin_whitelist(
        &request.owner,
        &request.repository,
        effective_sha,
        &manifest.version,
        &whitelist,
    )
    .map_err(Error::WhitelistRegex)?;

    if !allowed && !request.allow_untrusted {
        return Err(Error::NotWhitelisted {
            kind: "plugin",
            owner: request.owner.clone(),
            repository: request.repository.clone(),
            commit_sha: effective_sha.to_string(),
            version: manifest.version.clone(),
        });
    }

    let source = crate::filesystem::install::raw_manifest_url(
        &request.owner,
        &request.repository,
        request.commit_sha.as_deref(),
    );
    let installed = scoped
        .filesystem
        .install_plugin_from_manifest(
            &request.owner,
            &request.repository,
            &manifest,
            &source,
            None,
            false,
        )
        .await?;
    Ok(Response { installed })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::plugins::install::github as sdk;
    use objectiveai_sdk::cli::command::plugins::install::github::request_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::plugins::install::github as sdk;
    use objectiveai_sdk::cli::command::plugins::install::github::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
