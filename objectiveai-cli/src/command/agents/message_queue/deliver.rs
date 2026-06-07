//! `agents message-queue deliver` — fan out `agents message`
//! against every BOUND target with pending queue rows under the
//! caller's hierarchy (inclusive + recursive). Calls run in
//! parallel; the response streams are interleaved via
//! `futures::stream::SelectAll` so chunks arrive as their
//! upstream produces them. Each yielded item is augmented with
//! the resolved target's `agent_instance_hierarchy` (and, when
//! the underlying queue row was Tag-addressed, the `agent_tag`).
//!
//! PENDING / ABSENT tag rows are filtered out at the SQL layer
//! by [`db::prompts::list_delivery_targets_async`]; the deliver
//! sweep doesn't see them, so the caller can revisit later once
//! tags BOUND-up.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::agent::completions::message::RichContent;
use objectiveai_sdk::cli::command::agents::instances::message::{
    MessageTarget, Path as MessagePath, Request as MessageRequest, RequestDangerousAdvanced,
    RequestMessage,
};
use objectiveai_sdk::cli::command::agents::message_queue::deliver::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;
use crate::filesystem::db::{self, prompts::DeliveryTarget};

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let parent = request
        .parent_agent_instance_hierarchy
        .unwrap_or_else(|| ctx.config.agent_instance_hierarchy.clone());

    let targets =
        db::prompts::list_delivery_targets_async(ctx.filesystem.clone(), parent).await?;

    if targets.is_empty() {
        return Ok(Box::pin(futures::stream::empty()));
    }

    // Fire `agents::message::execute` for every target concurrently.
    // `execute` returns the stream once the underlying subprocess is
    // ready to yield; `join_all` lets all those readiness handshakes
    // happen in parallel. Pre-stream Errs are queued as one-item
    // error streams so the consumer sees them inline with the
    // successful deliveries' items.
    let starts = targets.into_iter().map(|tgt| {
        let req = build_message_request(&tgt);
        let ctx = ctx.clone();
        async move {
            let result = crate::command::agents::instances::message::execute(&ctx, req).await;
            (tgt, result)
        }
    });
    let results = futures::future::join_all(starts).await;

    let mut select_all = futures::stream::SelectAll::new();
    for (tgt, result) in results {
        match result {
            Ok(stream) => {
                let hier = tgt.agent_instance_hierarchy;
                let tag = tgt.agent_tag;
                let attributed = stream.map(move |r| {
                    r.map(|item| ResponseItem {
                        agent_instance_hierarchy: hier.clone(),
                        agent_tag: tag.clone(),
                        item,
                    })
                });
                select_all.push(Box::pin(attributed) as ItemStream);
            }
            Err(e) => {
                let once: ItemStream =
                    Box::pin(futures::stream::once(async move { Err(e) }));
                select_all.push(once);
            }
        }
    }

    Ok(Box::pin(select_all))
}

/// Build an in-process `agents::message::Request` for one delivery
/// target. Always Direct mode (the resolved hierarchy is already
/// known); the binding tag, if any, travels along so
/// `agents/message` rebinds it as a side effect — same shape a
/// user `agents message <leaf> --agent-tag <tag>` would take.
fn build_message_request(tgt: &DeliveryTarget) -> MessageRequest {
    let (parent, leaf) = match tgt.agent_instance_hierarchy.rfind('/') {
        Some(i) => (
            tgt.agent_instance_hierarchy[..i].to_string(),
            tgt.agent_instance_hierarchy[i + 1..].to_string(),
        ),
        // Root-level hierarchy (no '/'). Treat the whole string as the
        // leaf and use an empty parent — `agents/message` will compose
        // `format!("{parent}/{agent_instance}")` which yields
        // `format!("/{leaf}")`. In practice every CLI hierarchy starts
        // with `cli`, so this branch should not fire; defensively
        // still produces a usable shape.
        None => (String::new(), tgt.agent_instance_hierarchy.clone()),
    };
    MessageRequest {
        path_type: MessagePath::AgentsInstancesMessage,
        target: MessageTarget::Direct {
            parent_agent_instance_hierarchy: Some(parent),
            agent_instance: leaf,
            agent_tag: tgt.agent_tag.clone(),
        },
        // Empty Inline content — `agents/message` handles the
        // drain-only path (skips appending the empty own_content,
        // no-ops if drain is also empty).
        message: RequestMessage::Inline(RichContent::Text(String::new())),
        seed: None,
        // Stream chunks back so the deliver consumer sees the
        // agent's actual output, not just the head Queued ack.
        dangerous_advanced: Some(RequestDangerousAdvanced { stream: Some(true) }),
        jq: None,
    }
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::message_queue::deliver as sdk;
    use objectiveai_sdk::cli::command::agents::message_queue::deliver::request_schema::{
        Request, Response,
    };

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Request),
        ))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::message_queue::deliver as sdk;
    use objectiveai_sdk::cli::command::agents::message_queue::deliver::response_schema::{
        Request, Response,
    };

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::ResponseItem),
        ))
    }
}
