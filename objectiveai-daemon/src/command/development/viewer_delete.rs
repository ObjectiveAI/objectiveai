//! `development plugins viewer delete` — drop a viewer development
//! registration and bounce a running viewer, which comes back
//! resolving the installed copy. Same respawn-is-propagation story as
//! `viewer_create`.

use objectiveai_sdk::cli::command::development::plugins::viewer::delete::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(
    global: &GlobalContext,
    scoped: &ScopedContext,
    request: Request,
) -> Result<Response, Error> {
    let viewer_was_running = global.server_child_alive("viewer");
    let hubs = global.resident_hubs().ok_or_else(|| {
        Error::Development(
            "development plugins viewer delete requires the resident daemon".to_string(),
        )
    })?;

    let key = super::registry::key(&request.owner, &request.name, &request.version);
    // Removing nothing is a success: the caller asked for this plugin
    // not to be in development mode, and it is not.
    let removed = hubs.development_plugins.viewer.remove(&key).is_some();

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
        removed,
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::development::plugins::viewer::delete as sdk;
    use objectiveai_sdk::cli::command::development::plugins::viewer::delete::request_schema::{
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
    use objectiveai_sdk::cli::command::development::plugins::viewer::delete as sdk;
    use objectiveai_sdk::cli::command::development::plugins::viewer::delete::response_schema::{
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
