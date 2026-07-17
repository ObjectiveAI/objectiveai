//! `tools get` — read one tool's manifest by `(owner, name, version)`.
//! Returns `None` if not present.

use objectiveai_sdk::cli::command::tools::get::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(_global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<Response, Error> {
    Ok(scoped
        .filesystem
        .get_tool(&request.owner, &request.name, &request.version)
        .await
        .map(Into::into))
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::tools::get as sdk;
    use objectiveai_sdk::cli::command::tools::get::request_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::tools::get as sdk;
    use objectiveai_sdk::cli::command::tools::get::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
