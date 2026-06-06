//! `agents spawn` — bare-naked chunk-or-id streaming handler.
//!
//! Spawns the instance runner as a detached background process. The
//! streamed item shape depends on `dangerous_advanced.stream`:
//!
//! - **`None | Some(false)` (default)** — yields exactly one
//!   `ResponseItem::Id(<agent_instance_hierarchy>)` once the runner's
//!   `LogStreamReady` handshake fires, then the leaf's stream ends.
//!   The instance runner child keeps running orphaned and drives the
//!   completion to completion.
//! - **`Some(true)`** — yields the same `ResponseItem::Id` first, then
//!   one `ResponseItem::Chunk` per chunk Notification the runner
//!   emits, until the runner's stdout EOFs.
//!
//! `params.stream` on the wire is always `Some(true)` regardless of the
//! `dangerous_advanced.stream` field — the latter controls the leaf's
//! output stream behaviour, not the API request.

use std::path::PathBuf;
use std::pin::Pin;

use futures::Stream;
use futures::StreamExt;
use objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
use objectiveai_sdk::agent::completions::message::{
    Message, RichContent, UserMessage,
};
use objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams;
use objectiveai_sdk::cli::command::agents::spawn::{
    AgentSpec, Request, RequestPrompt, ResponseItem,
};

use crate::context::Context;
use crate::error::Error;
use crate::streaming::{InstanceItem, instance_subprocess_stream};

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let messages = resolve_prompt(request.prompt)?;
    let agent = resolve_agent(ctx, request.agent).await?;

    let params = AgentCompletionCreateParams {
        messages,
        provider: None,
        agent,
        response_format: None,
        seed: request.seed,
        stream: Some(true),
        continuation: None,
    };

    let stream = request
        .dangerous_advanced
        .as_ref()
        .and_then(|a| a.stream)
        .unwrap_or(false);

    let raw = instance_subprocess_stream(
        ctx,
        crate::instance::request::InstanceEndpoint::AgentsSpawn(params),
        None,
        request.agent_tag,
        stream,
    );
    Ok(Box::pin(raw.map(map_item)))
}

fn map_item(item: Result<InstanceItem, Error>) -> Result<ResponseItem, Error> {
    match item? {
        InstanceItem::Id(id) => Ok(ResponseItem::Id(id)),
        InstanceItem::Chunk(value) => serde_json::from_value(value)
            .map(ResponseItem::Chunk)
            .map_err(|e| Error::InlineJson(e)),
    }
}

pub(crate) fn resolve_prompt(prompt: RequestPrompt) -> Result<Vec<Message>, Error> {
    match prompt {
        RequestPrompt::Inline(msgs) => Ok(msgs),
        RequestPrompt::Simple(text) => Ok(vec![Message::User(UserMessage {
            content: RichContent::Text(text),
            name: None,
        })]),
        RequestPrompt::File(path) => read_messages_file(path),
        RequestPrompt::PythonInline(code) => crate::python::exec_code(&code),
        RequestPrompt::PythonFile(path) => crate::python::exec_file(&path),
    }
}

pub(crate) fn read_messages_file(path: PathBuf) -> Result<Vec<Message>, Error> {
    let bytes = std::fs::read(&path)
        .map_err(|e| Error::PromptFileRead(path.clone(), e))?;
    let mut de = serde_json::Deserializer::from_slice(&bytes);
    serde_path_to_error::deserialize(&mut de)
        .map_err(Error::InlineDeserialize)
}

async fn resolve_agent(
    ctx: &Context,
    spec: AgentSpec,
) -> Result<InlineAgentBaseWithFallbacksOrRemoteCommitOptional, Error> {
    match spec {
        AgentSpec::Resolved(resolved) => Ok(resolved),
        AgentSpec::Favorite(name) => {
            let mut config = ctx.filesystem.read_config().await?;
            let favorites = config.agents().get_favorites();
            let fav = favorites
                .iter()
                .find(|f| f.get_name() == name)
                .ok_or_else(|| Error::FavoriteNotFound(name.clone()))?;
            Ok(InlineAgentBaseWithFallbacksOrRemoteCommitOptional::Remote(
                fav.path.clone(),
            ))
        }
    }
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::spawn as sdk;
    use objectiveai_sdk::cli::command::agents::spawn::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::spawn as sdk;
    use objectiveai_sdk::cli::command::agents::spawn::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
