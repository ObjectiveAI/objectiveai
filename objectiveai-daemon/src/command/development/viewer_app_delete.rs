//! `development viewer delete` — back to the installed viewer.
//!
//! Clears the slot and bounces a RUNNING viewer, which relaunches as
//! the installed binary. Same respawn-is-propagation and
//! fatal-on-spawn-failure semantics as `viewer_app_set`.

use objectiveai_sdk::cli::command::development::viewer::delete::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(
    global: &GlobalContext,
    scoped: &ScopedContext,
    _request: Request,
) -> Result<Response, Error> {
    let hubs = global.resident_hubs().ok_or_else(|| {
        Error::Development(
            "development viewer delete requires the resident daemon".to_string(),
        )
    })?;

    let viewer_was_running = global.server_child_alive("viewer");
    // Clearing nothing is a success: the requested state ("the viewer
    // runs installed") already holds — and skips the bounce, since
    // nothing changed.
    let removed = hubs.development_plugins.viewer_app.clear().is_some();
    if removed {
        crate::command::kill_helpers::respawn_running_viewer(
            global,
            scoped,
            viewer_was_running,
        )
        .await?;
    }

    Ok(Response { removed })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::development::viewer::delete as sdk;
    use objectiveai_sdk::cli::command::development::viewer::delete::request_schema::{
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
    use objectiveai_sdk::cli::command::development::viewer::delete as sdk;
    use objectiveai_sdk::cli::command::development::viewer::delete::response_schema::{
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
