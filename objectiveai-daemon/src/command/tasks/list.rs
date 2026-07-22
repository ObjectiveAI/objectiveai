//! `tasks list` — a point-in-time row scan streamed one
//! [`ResponseItem`] per task (completed tasks included — they stay
//! listed until deleted). A stored command that no longer parses as
//! the current request type (a pre-wire-change row) yields an error
//! ITEM for that task; the stream continues.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::tasks::list::{LastResult, Request, ResponseItem};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(
    global: &GlobalContext,
    _scoped: &ScopedContext,
    _request: Request,
) -> Result<ItemStream, Error> {
    let db = global.db_client().await?;
    let rows = crate::db::tasks::list_tasks(&db).await?;
    let stream = async_stream::stream! {
        for row in rows {
            let command: objectiveai_sdk::cli::command::Request =
                match serde_json::from_value(row.command) {
                    Ok(command) => command,
                    Err(e) => {
                        yield Err(Error::Task(format!(
                            "task {}: stored command no longer parses: {e}",
                            row.id,
                        )));
                        continue;
                    }
                };
            let last_result = match row.last_result.as_deref() {
                Some("success") => Some(LastResult::Success),
                Some("error") => Some(LastResult::Error),
                _ => None,
            };
            let identity = row.agent_arguments;
            yield Ok(ResponseItem {
                id: row.id,
                command,
                delay_secs: row.delay_secs.max(0) as u64,
                repeat: row.repeat,
                repeat_count: row.repeat_count.map(|c| c.max(0) as u64),
                run_count: row.run_count.max(0) as u64,
                error_count: row.error_count.max(0) as u64,
                last_result,
                complete: row.complete,
                created_at: crate::db::time::unix_to_rfc3339(row.created_at),
                next_run_at: row.next_run_at.map(crate::db::time::unix_to_rfc3339),
                agent_instance_hierarchy: identity.agent_instance_hierarchy,
                plugin_owner: identity.plugin_owner,
                plugin_name: identity.plugin_name,
                plugin_version: identity.plugin_version,
            });
        }
    };
    Ok(Box::pin(stream))
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::tasks::list as sdk;
    use objectiveai_sdk::cli::command::tasks::list::request_schema::{Request, Response};

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
    use objectiveai_sdk::cli::command::tasks::list as sdk;
    use objectiveai_sdk::cli::command::tasks::list::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(
        _global: &GlobalContext,
        _scoped: &ScopedContext,
        _request: Request,
    ) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::ResponseItem)))
    }
}
