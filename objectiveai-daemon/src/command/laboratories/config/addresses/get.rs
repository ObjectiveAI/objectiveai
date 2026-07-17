//! `laboratories config addresses get` — read `laboratories.addresses`
//! from on-disk config.

use objectiveai_sdk::cli::command::laboratories::config::addresses::get::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(_global: &GlobalContext, scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
    let mut config = scoped.filesystem.read_config().await?;
    Ok(Response {
        addresses: config.laboratories().get_addresses().cloned(),
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::laboratories::config::addresses::get as sdk;
    use objectiveai_sdk::cli::command::laboratories::config::addresses::get::request_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::laboratories::config::addresses::get as sdk;
    use objectiveai_sdk::cli::command::laboratories::config::addresses::get::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
