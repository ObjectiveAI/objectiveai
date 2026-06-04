//! `functions publish` — write a `FullRemoteFunction` to a repository
//! on the filesystem and return its content sha.

use objectiveai_sdk::cli::command::functions::publish::{
    Request, RequestBody, RequestPublishMessage, Response,
};
use objectiveai_sdk::functions::FullRemoteFunction;

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    let body = resolve_body(request.body)?;
    let message = resolve_publish_message(request.message)?;
    let sha = crate::filesystem::publish::publish_function(
        &ctx.filesystem,
        &request.repository,
        &body,
        &message,
        request.overwrite,
    )
    .await?;
    Ok(Response { sha })
}

fn resolve_body(body: RequestBody) -> Result<FullRemoteFunction, Error> {
    match body {
        RequestBody::Inline(v) => Ok(v),
        RequestBody::File(p) => crate::source_resolver::resolve_source(
            None, None, Some(p), None, None,
            |_| unreachable!(),
        ),
        RequestBody::PythonInline(s) => crate::source_resolver::resolve_source(
            None, None, None, Some(s), None,
            |_| unreachable!(),
        ),
        RequestBody::PythonFile(p) => crate::source_resolver::resolve_source(
            None, None, None, None, Some(p),
            |_| unreachable!(),
        ),
    }
}

fn resolve_publish_message(m: RequestPublishMessage) -> Result<String, Error> {
    match m {
        RequestPublishMessage::Inline(s) => Ok(s),
        RequestPublishMessage::File(p) => std::fs::read_to_string(&p)
            .map_err(|e| Error::PromptFileRead(p, e)),
    }
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::functions::publish as sdk;
    use objectiveai_sdk::cli::command::functions::publish::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::functions::publish as sdk;
    use objectiveai_sdk::cli::command::functions::publish::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
