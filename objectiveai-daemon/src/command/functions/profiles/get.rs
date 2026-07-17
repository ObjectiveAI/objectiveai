//! `functions profiles get` — read a profile definition by remote
//! path.

use objectiveai_sdk::cli::command::functions::profiles::get::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<Response, Error> {
    crate::retrieve::get_profile(global, scoped, request.path).await
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::functions::profiles::get as sdk;
    use objectiveai_sdk::cli::command::functions::profiles::get::request_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::functions::profiles::get as sdk;
    use objectiveai_sdk::cli::command::functions::profiles::get::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
