//! `tools install github` — fetch the manifest, check the whitelist,
//! and install the tool under
//! `~/.objectiveai/bin/tools/<owner>/<repository>/<version>/`. The
//! bare-naked contract surfaces the untrusted decision as the typed
//! `Error::NotWhitelisted { kind: "tool", .. }` variant.
//!
//! The SDK `Request` does not expose `upgrade`; this leaf always
//! installs fresh and surfaces `Error::AlreadyInstalled` if a
//! manifest already exists.

use objectiveai_sdk::cli::command::tools::install::github::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    let manifest = ctx
        .filesystem
        .fetch_tool_manifest(
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
            kind: "tool",
            owner: request.owner.clone(),
            repository: request.repository.clone(),
            commit_sha: effective_sha.to_string(),
            version: manifest.version.clone(),
        });
    }

    let installed = ctx
        .filesystem
        .install_tool_from_manifest(
            &request.owner,
            &request.repository,
            &manifest,
            None,
            false,
        )
        .await?;
    Ok(Response { installed })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::tools::install::github as sdk;
    use objectiveai_sdk::cli::command::tools::install::github::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::tools::install::github as sdk;
    use objectiveai_sdk::cli::command::tools::install::github::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
