//! `python` — run a Python snippet in the embedded runtime and return
//! its output as JSON.
//!
//! `--code` is executed with the optional `--input` JSON value exposed
//! as the global `input` (the same harness the `--python` output
//! transform uses). The script's output — its trailing expression
//! value, else captured stdout — is returned as a `serde_json::Value`;
//! a script that produces no output yields JSON `null`.

use objectiveai_sdk::cli::command::python::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<Response, Error> {
    // `--no-objectiveai` gates the in-process `objectiveai.execute` host call.
    let scoped = scoped.with_no_objectiveai(request.no_objectiveai.unwrap_or(false));
    let output: Option<serde_json::Value> = global
        .python()
        .await?
        .exec_code(global, &scoped, &request.code, request.input)
        .await?;
    Ok(output.unwrap_or(serde_json::Value::Null))
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::python as sdk;
    use objectiveai_sdk::cli::command::python::request_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::python as sdk;
    use objectiveai_sdk::cli::command::python::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
