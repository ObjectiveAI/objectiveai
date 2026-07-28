//! `development viewer set` — run the viewer from a source checkout.
//!
//! Writes the singleton slot, then bounces a RUNNING viewer into the
//! new form via [`crate::command::kill_helpers::respawn_running_viewer`]
//! — which is where the user's contract lands: the respawn is FATAL on
//! spawn failure, so a source tree that fails to start (a cargo or
//! vite error) FAILS THIS COMMAND, with the build output riding the
//! spawn error. An absent viewer is not launched; every future spawn
//! reads the slot fresh.

use objectiveai_sdk::cli::command::development::viewer::set::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(
    global: &GlobalContext,
    scoped: &ScopedContext,
    request: Request,
) -> Result<Response, Error> {
    let hubs = global.resident_hubs().ok_or_else(|| {
        Error::Development(
            "development viewer set requires the resident daemon".to_string(),
        )
    })?;

    // Sampled BEFORE the write, like every viewer-bouncing mutation.
    let viewer_was_running = global.server_child_alive("viewer");

    // Absolute + directory, checked here so the mistake surfaces when
    // it is made. NOT checked: pnpm, node_modules, tauri.conf — a bad
    // checkout fails at spawn, loudly, which is the contract.
    let path = std::path::PathBuf::from(&request.path);
    if !path.is_absolute() {
        return Err(Error::Development(format!(
            "--path must be absolute, got {:?}",
            request.path
        )));
    }
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

    let replaced = hubs.development_plugins.viewer_app.set(path).is_some();

    crate::command::kill_helpers::respawn_running_viewer(
        global,
        scoped,
        viewer_was_running,
    )
    .await?;

    Ok(Response {
        path: request.path,
        replaced,
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::development::viewer::set as sdk;
    use objectiveai_sdk::cli::command::development::viewer::set::request_schema::{
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
    use objectiveai_sdk::cli::command::development::viewer::set as sdk;
    use objectiveai_sdk::cli::command::development::viewer::set::response_schema::{
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
