//! `development plugins viewer create` — register a local directory
//! for a plugin's VIEWER half, then bounce a running viewer onto it.
//!
//! The registrations ride the viewer's argv, frozen at spawn, so
//! respawn IS the propagation: a running viewer is killed and
//! respawned with the new list; an absent one is NOT spawned (a
//! registration never turns into a surprise viewer launch) but picks
//! the list up at its next spawn, which always reads the registry
//! fresh.

use objectiveai_sdk::cli::command::development::plugins::viewer::create::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(
    global: &GlobalContext,
    scoped: &ScopedContext,
    request: Request,
) -> Result<Response, Error> {
    // Sampled BEFORE the mutation, like every viewer-bouncing
    // mutation: only a viewer the user already had up gets bounced.
    let viewer_was_running = global.server_child_alive("viewer");
    let hubs = global.resident_hubs().ok_or_else(|| {
        Error::Development(
            "development plugins viewer create requires the resident daemon".to_string(),
        )
    })?;

    // Absolute, because the LABORATORY HOST resolves this path, and its
    // working directory is neither this process's nor the caller's.
    // Rejected here rather than at build time so the mistake surfaces
    // when it is made, not on the next completion.
    let path = std::path::PathBuf::from(&request.path);
    if !path.is_absolute() {
        return Err(Error::Development(format!(
            "--path must be absolute, got {:?}",
            request.path
        )));
    }
    // Existence is checked, but NOT the manifest: a directory that is
    // about to have an `objectiveai.json` written into it is a
    // perfectly reasonable thing to register, and the host reports a
    // missing or invalid manifest with its own error code when it
    // actually goes to build.
    match tokio::fs::metadata(&path).await {
        Ok(meta) if meta.is_dir() => {}
        Ok(_) => {
            return Err(Error::Development(format!(
                "--path {:?} is not a directory",
                request.path
            )));
        }
        Err(e) => {
            return Err(Error::Development(format!("--path {:?}: {e}", request.path)));
        }
    }

    let key = super::registry::key(&request.owner, &request.name, &request.version);
    let replaced = hubs.development_plugins.viewer.insert(key.clone(), path).is_some();

    crate::command::kill_helpers::respawn_running_viewer(
        global,
        scoped,
        viewer_was_running,
    )
    .await?;

    Ok(Response {
        owner: key.0,
        name: key.1,
        version: key.2,
        path: request.path,
        replaced,
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::development::plugins::viewer::create as sdk;
    use objectiveai_sdk::cli::command::development::plugins::viewer::create::request_schema::{
        Request, Response,
    };

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(
        _global: &GlobalContext,
        _scoped: &ScopedContext,
        _request: Request,
    ) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Request),
        ))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::development::plugins::viewer::create as sdk;
    use objectiveai_sdk::cli::command::development::plugins::viewer::create::response_schema::{
        Request, Response,
    };

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(
        _global: &GlobalContext,
        _scoped: &ScopedContext,
        _request: Request,
    ) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Response),
        ))
    }
}
