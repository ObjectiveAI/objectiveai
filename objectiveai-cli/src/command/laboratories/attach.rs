//! `laboratories attach` — insert the `(target, laboratory_id)`
//! row. Errors if the laboratory is already attached to the target.
//! NO LOCKING: attaching works at any time, active agents included —
//! the spawn picks the change up at its next pass boundary.

use objectiveai_sdk::cli::command::laboratories::attach::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    let target = super::resolve_target(ctx, &request.selector).await?;
    let pool = ctx.db_client().await?.clone();
    let inserted = crate::db::laboratory_attachments::attach(
        &pool,
        &target,
        &request.laboratory_id,
        &ctx.config.agent_instance_hierarchy,
    )
    .await?;
    if !inserted {
        return Err(Error::LaboratoryAlreadyAttached {
            laboratory_id: request.laboratory_id,
        });
    }
    Ok(Response {
        laboratory_id: request.laboratory_id,
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::laboratories::attach as sdk;
    use objectiveai_sdk::cli::command::laboratories::attach::request_schema::{
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
    use objectiveai_sdk::cli::command::laboratories::attach as sdk;
    use objectiveai_sdk::cli::command::laboratories::attach::response_schema::{
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
