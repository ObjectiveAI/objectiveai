//! `functions publish` — write a `FullRemoteFunction` to a repository
//! on the filesystem and return its content sha.

use objectiveai_sdk::cli::command::functions::publish::{
    Request, RequestBody, RequestPublishMessage, Response,
};
use objectiveai_sdk::functions::FullRemoteFunction;

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<Response, Error> {
    let body = resolve_body(global, scoped, request.body).await?;
    let message = resolve_publish_message(request.message)?;
    let sha = crate::filesystem::publish::publish_function(
        &scoped.filesystem,
        &request.repository,
        &body,
        &message,
        request.overwrite,
    )
    .await?;
    Ok(Response { sha })
}

async fn resolve_body(global: &GlobalContext, scoped: &ScopedContext, body: RequestBody) -> Result<FullRemoteFunction, Error> {
    match body {
        RequestBody::Inline(v) => Ok(v),
        RequestBody::File(p) => crate::source_resolver::resolve_source(
            global, scoped, None, None, Some(p), None, None,
            |_| unreachable!(),
        ).await,
        RequestBody::PythonInline(s) => crate::source_resolver::resolve_source(
            global, scoped, None, None, None, Some(s), None,
            |_| unreachable!(),
        ).await,
        RequestBody::PythonFile(p) => crate::source_resolver::resolve_source(
            global, scoped, None, None, None, None, Some(p),
            |_| unreachable!(),
        ).await,
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

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::functions::publish as sdk;
    use objectiveai_sdk::cli::command::functions::publish::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
