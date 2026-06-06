//! `agents me` — bare-naked handler stub.

use objectiveai_sdk::cli::command::agents::me::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, _request: Request) -> Result<Response, Error> {
    Ok(Response {
        agent_instance_hierarchy: ctx.config.agent_instance_hierarchy.clone(),
        agent_id: ctx.config.agent_id.clone(),
        agent_full_id: ctx.config.agent_full_id.clone(),
        agent_remote: ctx.config.agent_remote.clone(),
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::me as sdk;
    use objectiveai_sdk::cli::command::agents::me::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::me as sdk;
    use objectiveai_sdk::cli::command::agents::me::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
