//! `agents publish` — write a `RemoteAgentBaseWithFallbacks` to a
//! repository on the filesystem and return its content sha.
//!
//! Body resolves via the 4-variant `RequestBody` enum (inline /
//! file / python-inline / python-file) and the commit message via
//! the 2-variant `RequestPublishMessage` (inline / file).

use objectiveai_sdk::agent::RemoteAgentBaseWithFallbacks;
use objectiveai_sdk::cli::command::agents::publish::{
    Request, RequestBody, RequestPublishMessage, Response,
};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<Response, Error> {
    let body = resolve_body(global, scoped, request.body).await?;
    let message = resolve_publish_message(request.message)?;
    let sha = crate::filesystem::publish::publish_agent(
        &scoped.filesystem,
        &request.repository,
        &body,
        &message,
        request.overwrite,
    )
    .await?;
    Ok(Response { sha })
}

async fn resolve_body(
    global: &GlobalContext, scoped: &ScopedContext,
    body: RequestBody,
) -> Result<RemoteAgentBaseWithFallbacks, Error> {
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
    use objectiveai_sdk::cli::command::agents::publish as sdk;
    use objectiveai_sdk::cli::command::agents::publish::request_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::publish as sdk;
    use objectiveai_sdk::cli::command::agents::publish::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
