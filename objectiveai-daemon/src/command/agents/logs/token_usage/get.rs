//! `agents logs token-usage get` — read an agent's current stored
//! `total_tokens` snapshot. `total_tokens` is null when no
//! agent-completion usage has been recorded for the AIH yet.

use objectiveai_sdk::cli::command::agents::logs::token_usage::get::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(global: &GlobalContext, _scoped: &ScopedContext, request: Request) -> Result<Response, Error> {
    let db = global.db_client().await?;
    let total_tokens =
        crate::db::logs::get_agent_token_usage(&db, &request.agent_instance_hierarchy).await?;
    Ok(Response {
        agent_instance_hierarchy: request.agent_instance_hierarchy,
        total_tokens,
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::logs::token_usage::get as sdk;
    use objectiveai_sdk::cli::command::agents::logs::token_usage::get::request_schema::{
        Request, Response,
    };

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::logs::token_usage::get as sdk;
    use objectiveai_sdk::cli::command::agents::logs::token_usage::get::response_schema::{
        Request, Response,
    };

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
