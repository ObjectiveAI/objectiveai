//! `agents tasks run` — fire every pending schedule in scope.
//!
//! Resolves the scope (literal hierarchy / tag lookup / default
//! to `ctx.config.agent_instance_hierarchy`), captures every
//! pending row via the transactional
//! `db::tasks::collect_and_mark_pending_async` (which also bumps
//! `last_ran_at` and deletes oneshots upfront), then dispatches
//! each row's stored argv through the in-process root command
//! dispatcher in parallel. Per-task streams are merged via
//! [`futures::stream::SelectAll`]; each item is wrapped with the
//! source schedule's `name` so callers can attribute output.
//!
//! Why we don't use `CliCommandExecutor`'s generic
//! `execute<R, T>`: the trait method's hidden async type goes
//! through `crate::command::command::execute`, which dispatches
//! back to leaves (including this one), creating a type-inference
//! cycle. We inline what the executor does — `parse_request` +
//! per-task ctx override + `command::command::execute` — so the
//! recursion lives in runtime control flow, not type inference.
//! And because we already have the typed root `ResponseItem` out
//! of the local dispatcher, we skip the serde_json round-trip
//! that the generic `execute<_, T>` uses to land on `T`.
//!
//! Pre-stream `Err`s (e.g. the scheduled command failed to
//! parse) are re-emitted as a single-item error stream in the
//! merged output, matching the deliver leaf's pattern.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::AgentArguments;
use objectiveai_sdk::cli::command::ResponseItem as RootResponseItem;
use objectiveai_sdk::cli::command::agents::tasks::run::{Request, ResponseItem};
use objectiveai_sdk::cli::command::parse_request;

use crate::context::Context;
use crate::db;
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let parent = super::resolve_scope(ctx, request.agent_instance_hierarchy, request.tag).await?;

    let rows = db::tasks::collect_and_mark_pending(
        &ctx.db,
        &parent,
        request.depth,
    )
    .await?;

    if rows.is_empty() {
        return Ok(Box::pin(futures::stream::empty()));
    }

    // Kick all per-task dispatches off in parallel. Each future
    // resolves to `(id, Result<RootStream, Error>)` where `id` is
    // the wire-shape `"{name}-{db_id}"` — pre-stream errors
    // (parse / handler-rejection) are folded into the merged
    // stream as one-item error streams below.
    let starts = rows.into_iter().map(|row| {
        let ctx = ctx.clone();
        async move {
            let id = format!("{}-{}", row.name, row.id);
            let stream_result = run_one(&ctx, row.command, &row.agent_arguments).await;
            (id, stream_result)
        }
    });
    let results = futures::future::join_all(starts).await;

    let mut select_all = futures::stream::SelectAll::new();
    for (id, result) in results {
        match result {
            Ok(stream) => {
                let tag = id;
                let tagged = stream.map(move |r| {
                    r.map(|value| ResponseItem {
                        id: tag.clone(),
                        value: Box::new(value),
                    })
                });
                select_all.push(Box::pin(tagged) as ItemStream);
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

/// Per-task stream — yields the typed root `ResponseItem` that
/// `crate::command::command::execute` already produces. No JSON
/// round-trip: the executor's `to_value` + `extract_leaf` dance
/// exists only to let the generic `execute<_, T>` settle on `T`,
/// and our `T` here *is* the root union, so we keep the value
/// typed end-to-end.
type RootStream = Pin<Box<dyn Stream<Item = Result<RootResponseItem, Error>> + Send>>;

/// Dispatch one stored schedule. Mirrors
/// `CliCommandExecutor::execute<_, RootResponseItem>` with the
/// serde round-trip dropped — see [`RootStream`].
async fn run_one(
    ctx: &Context,
    argv: Vec<String>,
    agent_arguments: &AgentArguments,
) -> Result<RootStream, Error> {
    let sdk_request = parse_request(&argv).map_err(|e| match e {
        objectiveai_sdk::cli::command::ParseError::Clap(e) => Error::ClapParse(e),
        objectiveai_sdk::cli::command::ParseError::FromArgs(e) => Error::FromArgs(e),
    })?;
    let task_ctx = apply_agent_arguments(ctx, agent_arguments);
    let stream = crate::command::command::execute(&task_ctx, sdk_request).await?;
    Ok(Box::pin(stream))
}

/// Clone `ctx` and overwrite the seven identity fields on its
/// `Config` from the schedule's saved `AgentArguments`. Mirrors
/// `CliCommandExecutor::resolve_ctx`'s Some-arm — including the
/// `"UNKNOWN"` fallback for the non-nullable
/// `agent_instance_hierarchy` field.
fn apply_agent_arguments(ctx: &Context, args: &AgentArguments) -> Context {
    let mut ctx = ctx.clone();
    ctx.config.agent_instance_hierarchy = args
        .agent_instance_hierarchy
        .clone()
        .unwrap_or_else(|| "UNKNOWN".to_string());
    ctx.config.agent_id = args.agent_id.clone();
    ctx.config.agent_full_id = args.agent_full_id.clone();
    ctx.config.agent_remote = args.agent_remote.clone();
    ctx.config.response_id = args.response_id.clone();
    ctx.config.response_ids = args.response_ids.clone();
    ctx.config.mcp_session_id = args.mcp_session_id.clone();
    ctx
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::tasks::run as sdk;
    use objectiveai_sdk::cli::command::agents::tasks::run::request_schema::{
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
    use objectiveai_sdk::cli::command::agents::tasks::run as sdk;
    use objectiveai_sdk::cli::command::agents::tasks::run::response_schema::{
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
