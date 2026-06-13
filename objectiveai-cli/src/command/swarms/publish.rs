//! `swarms publish` — write a `RemoteSwarmBase` to the local filesystem
//! under `<base>/swarms/<repository>` and return the resulting git
//! commit SHA. Body and commit message can be supplied inline, from a
//! JSON file, or produced by Python (the embedded WASI rustpython).

use objectiveai_sdk::cli::command::swarms::publish::{
    Request, RequestBody, RequestPublishMessage, Response,
};
use objectiveai_sdk::swarm::RemoteSwarmBase;

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    let Request {
        repository,
        body,
        message,
        overwrite,
        ..
    } = request;

    let swarm: RemoteSwarmBase = resolve_body(ctx, body).await?;
    let msg = resolve_message(message)?;
    let sha = crate::filesystem::publish::publish_swarm(
        &ctx.filesystem,
        &repository,
        &swarm,
        &msg,
        overwrite,
    )
    .await?;
    Ok(Response { sha })
}

async fn resolve_body(ctx: &Context, body: RequestBody) -> Result<RemoteSwarmBase, Error> {
    match body {
        RequestBody::Inline(swarm) => Ok(swarm),
        RequestBody::File(path) => {
            let contents = std::fs::read_to_string(&path)
                .map_err(|e| Error::PromptFileRead(path, e))?;
            let mut de = serde_json::Deserializer::from_str(&contents);
            serde_path_to_error::deserialize(&mut de).map_err(Error::InlineDeserialize)
        }
        RequestBody::PythonInline(code) => ctx.python().await?.exec_code(&code, None::<()>).await?.ok_or(Error::PythonNoOutput),
        RequestBody::PythonFile(path) => ctx.python().await?.exec_file(&path, None::<()>).await?.ok_or(Error::PythonNoOutput),
    }
}

fn resolve_message(message: RequestPublishMessage) -> Result<String, Error> {
    match message {
        RequestPublishMessage::Inline(s) => Ok(s),
        RequestPublishMessage::File(path) => std::fs::read_to_string(&path)
            .map_err(|e| Error::PromptFileRead(path, e)),
    }
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::swarms::publish as sdk;
    use objectiveai_sdk::cli::command::swarms::publish::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::swarms::publish as sdk;
    use objectiveai_sdk::cli::command::swarms::publish::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
