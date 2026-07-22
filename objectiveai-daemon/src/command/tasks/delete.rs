//! `tasks delete` — remove by id, idempotent. A task deleted mid-run
//! finishes its in-flight run; the run's completion UPDATE then
//! matches nothing (no resurrection) and the task never fires again.

use objectiveai_sdk::cli::command::tasks::delete::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(
    global: &GlobalContext,
    scoped: &ScopedContext,
    request: Request,
) -> Result<Response, Error> {
    let hubs = global.resident_hubs().ok_or_else(|| {
        Error::Task("tasks delete requires the resident daemon".to_string())
    })?;
    let db = global.db_client().await?;
    // The id resolves within the CALLER's plugin namespace — a plugin
    // can only delete its own tasks' ids, a non-plugin caller only
    // the all-NULL namespace. Plain identity scoping, not auth.
    let deleted = crate::db::tasks::delete_task(
        &db,
        (
            scoped.plugin_owner(),
            scoped.plugin_name(),
            scoped.plugin_version(),
        ),
        &request.id,
    )
    .await?;
    hubs.tasks.notify();
    Ok(Response { deleted })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::tasks::delete as sdk;
    use objectiveai_sdk::cli::command::tasks::delete::request_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(
        _global: &GlobalContext,
        _scoped: &ScopedContext,
        _request: Request,
    ) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::tasks::delete as sdk;
    use objectiveai_sdk::cli::command::tasks::delete::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(
        _global: &GlobalContext,
        _scoped: &ScopedContext,
        _request: Request,
    ) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
