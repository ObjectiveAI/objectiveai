//! `tasks create` — validate, persist, arm. The command arrives TYPED
//! (deserialization already validated it); create additionally
//! forbids the classes that must never run unattended:
//!
//! - `daemon …` — a scheduled `daemon kill` is a boot-loop suicide
//!   (the kill dies with the daemon mid-run; the boot reconcile
//!   re-arms it and it re-fires on every boot).
//! - `update` — self-update mid-flight, same crash-mid-run family.
//! - `tasks …` — tasks creating tasks is an unbounded fork bomb under
//!   `--repeat`.
//!
//! The stored identity is the CREATOR's whole scope identity (agent
//! arguments + plugin trio) — what the fired run reconstructs.

use objectiveai_sdk::cli::command::tasks::create::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(
    global: &GlobalContext,
    scoped: &ScopedContext,
    request: Request,
) -> Result<Response, Error> {
    // Wire callers bypass clap — every CLI-side rule re-checks here.
    if request.delay_secs < 1 {
        return Err(Error::Task("--delay-secs must be at least 1".to_string()));
    }
    match request.repeat_count {
        Some(_) if !request.repeat => {
            return Err(Error::Task(
                "--repeat-count requires --repeat".to_string(),
            ));
        }
        Some(0) => {
            return Err(Error::Task(
                "--repeat-count must be greater than 0".to_string(),
            ));
        }
        _ => {}
    }
    {
        use objectiveai_sdk::cli::command::Request as Cmd;
        match &*request.command {
            Cmd::Daemon(_) => {
                return Err(Error::Task(
                    "daemon commands may not be scheduled".to_string(),
                ));
            }
            Cmd::Update(_) | Cmd::UpdateRequestSchema(_) | Cmd::UpdateResponseSchema(_) => {
                return Err(Error::Task(
                    "update may not be scheduled".to_string(),
                ));
            }
            Cmd::Tasks(_) => {
                return Err(Error::Task(
                    "tasks commands may not be scheduled".to_string(),
                ));
            }
            _ => {}
        }
    }

    let hubs = global.resident_hubs().ok_or_else(|| {
        Error::Task("tasks create requires the resident daemon".to_string())
    })?;
    let db = global.db_client().await?;
    let id = uuid::Uuid::new_v4().to_string();
    let command = serde_json::to_value(&*request.command)
        .map_err(|e| Error::Task(format!("serialize command: {e}")))?;
    let identity = crate::command::channels::scope_identity(scoped);
    crate::db::tasks::insert_task(
        &db,
        &id,
        &command,
        &identity,
        request.delay_secs as i64,
        request.repeat,
        request.repeat_count.map(|c| c as i64),
    )
    .await?;
    // Write first, THEN wake the driver (notify stores a permit — no
    // lost wakeup).
    hubs.tasks.notify();
    Ok(Response { id })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::tasks::create as sdk;
    use objectiveai_sdk::cli::command::tasks::create::request_schema::{Request, Response};

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
    use objectiveai_sdk::cli::command::tasks::create as sdk;
    use objectiveai_sdk::cli::command::tasks::create::response_schema::{Request, Response};

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
