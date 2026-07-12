//! `laboratories detach` — delete the `(target, laboratory)` row,
//! addressed by the laboratory's FULL identity (id + the
//! `--machine`/`--machine-state` pair, auto-filled with the local
//! machine + the daemon's own state when neither is given). Errors if
//! that exact laboratory was not attached to the target. NO LOCKING:
//! detaching works at any time, active agents included — the spawn
//! picks the change up at its next pass boundary.

use objectiveai_sdk::cli::command::laboratories::detach::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    let (machine, machine_state) =
        super::resolve_pair(ctx, request.machine.clone(), request.machine_state.clone())?;
    let target = super::resolve_target(ctx, &request.selector).await?;
    let pool = ctx.db_client().await?.clone();
    let deleted = crate::db::laboratory_attachments::detach(
        &pool,
        &target,
        &request.laboratory_id,
        &machine,
        &machine_state,
    )
    .await?;
    if !deleted {
        return Err(Error::LaboratoryNotAttached {
            laboratory_id: request.laboratory_id,
        });
    }
    Ok(Response {
        laboratory_id: request.laboratory_id,
        machine: Some(machine),
        machine_state: Some(machine_state),
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::laboratories::detach as sdk;
    use objectiveai_sdk::cli::command::laboratories::detach::request_schema::{
        Request, Response,
    };

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Request),
        ))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::laboratories::detach as sdk;
    use objectiveai_sdk::cli::command::laboratories::detach::response_schema::{
        Request, Response,
    };

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Response),
        ))
    }
}
