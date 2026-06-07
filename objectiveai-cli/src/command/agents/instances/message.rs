//! `agents instances message` — pure enqueue.
//!
//! Persists one `RichContent` into the `prompts` table in
//! `tags.sqlite` against either the resolved `{parent}/{instance}`
//! hierarchy (Direct mode) or the literal tag name (Tag mode — no
//! resolution at enqueue time). The API picks the row up via the
//! WS reverse-attach `read_message_queue` predicate once a matching
//! hierarchy comes online.
//!
//! No pipe delivery, no continuation fallback, no model invocation.
//! Same wire shape as `agents message-queue add` — `id` of the new
//! row plus the chosen target.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::agent::completions::message::RichContent;
use objectiveai_sdk::cli::command::agents::instances::message::{
    MessageTarget, Request, RequestMessage, Response, ResponseItem,
};

use crate::context::Context;
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let content = resolve_message(request.message)?;

    // Direct: compose `{parent}/{agent_instance}` (parent defaults to
    // the cli's own position). No `agent_exists` check — `agents
    // message` is symmetrically lenient with `Tag`. The queue stores
    // the row against the resolved hierarchy; the API picks it up
    // via the `read_message_queue` predicate once a matching
    // hierarchy comes online.
    //
    // Tag: store the tag name verbatim. No lookup — the tag does NOT
    // need to exist yet. Tag-addressed rows resolve at read time
    // (rule 2: any BOUND tag mapping to the current hierarchy;
    // rule 3: an explicit tag application as part of a spawn).
    let (agent_instance_hierarchy, agent_tag) = match request.target {
        MessageTarget::Direct {
            parent_agent_instance_hierarchy,
            agent_instance,
        } => {
            let parent = parent_agent_instance_hierarchy
                .as_deref()
                .unwrap_or(&ctx.config.agent_instance_hierarchy);
            (Some(format!("{parent}/{agent_instance}")), None)
        }
        MessageTarget::Tag { agent_tag } => (None, Some(agent_tag)),
    };

    let id = crate::filesystem::db::prompts::enqueue_with_content_async(
        ctx.filesystem.clone(),
        agent_instance_hierarchy.clone(),
        agent_tag.clone(),
        // No `--key` on this leaf — idempotency tokens are exclusive
        // to `agents message-queue add`. Every enqueue here stacks a
        // new row.
        None,
        content,
    )
    .await?;

    let response = Response {
        id,
        agent_instance_hierarchy,
        agent_tag,
    };
    // `ResponseItem` is a type alias for `Response` (single-item
    // streamed shape mirrors the unary response).
    Ok(Box::pin(futures::stream::once(async move {
        Ok::<ResponseItem, Error>(response)
    })))
}

pub fn resolve_message(message: RequestMessage) -> Result<RichContent, Error> {
    let (simple, inline, file, python_inline, python_file) = match message {
        RequestMessage::Inline(rich) => return Ok(rich),
        RequestMessage::Simple(s) => (Some(s), None, None, None, None),
        RequestMessage::File(p) => (None, None, Some(p), None, None),
        RequestMessage::PythonInline(code) => (None, None, None, Some(code), None),
        RequestMessage::PythonFile(p) => (None, None, None, None, Some(p)),
    };
    crate::source_resolver::resolve_source(
        simple,
        inline,
        file,
        python_inline,
        python_file,
        RichContent::Text,
    )
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::instances::message as sdk;
    use objectiveai_sdk::cli::command::agents::instances::message::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::instances::message as sdk;
    use objectiveai_sdk::cli::command::agents::instances::message::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
