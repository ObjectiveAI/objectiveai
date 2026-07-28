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
    // Resolve the id's namespace: the caller's own plugin identity by
    // default, the all-NULL namespace on `--no-plugin`, or an explicit
    // trio. Plain identity resolution — a caller may name any
    // namespace (not authentication).
    use objectiveai_sdk::cli::command::tasks::delete::DeleteNamespace;
    let plugin = match &request.namespace {
        DeleteNamespace::Caller => (
            scoped.plugin_owner(),
            scoped.plugin_name(),
            scoped.plugin_version(),
        ),
        DeleteNamespace::NoPlugin => (None, None, None),
        DeleteNamespace::Plugin {
            owner,
            name,
            version,
        } => (Some(owner.as_str()), Some(name.as_str()), Some(version.as_str())),
    };
    let deleted = crate::db::tasks::delete_task(&db, plugin, &request.id).await?;
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
