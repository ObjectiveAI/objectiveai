//! `swarms get <path>` — fetch a swarm by its remote path. The SDK's
//! `TryFrom<Args>` has already parsed the docker-style
//! `key=value,...` string into a `RemotePathCommitOptional`.

use objectiveai_sdk::cli::command::swarms::get::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    let path = request.path;
    let response = objectiveai_sdk::swarm::get_swarm(ctx.api_client().await?, path).await?;
    Ok(response)
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::swarms::get as sdk;
    use objectiveai_sdk::cli::command::swarms::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::swarms::get as sdk;
    use objectiveai_sdk::cli::command::swarms::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
