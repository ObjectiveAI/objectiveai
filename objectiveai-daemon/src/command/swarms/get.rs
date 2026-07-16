//! `swarms get <path>` — fetch a swarm by its remote path. The SDK's
//! `TryFrom<Args>` has already parsed the docker-style
//! `key=value,...` string into a `RemotePathCommitOptional`.

use objectiveai_sdk::cli::command::swarms::get::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<Response, Error> {
    crate::retrieve::get_swarm(global, scoped, request.path).await
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::swarms::get as sdk;
    use objectiveai_sdk::cli::command::swarms::get::request_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::swarms::get as sdk;
    use objectiveai_sdk::cli::command::swarms::get::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
