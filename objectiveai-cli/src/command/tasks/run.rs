//! `agents tasks run` — fire every pending schedule in the caller's
//! own subtree.
//!
//! Scope is fixed to the caller's own AIH
//! (`ctx.config.agent_instance_hierarchy`) plus every descendant.
//! `db::tasks::collect_and_mark_pending` captures the pending rows
//! transactionally (bumping `last_ran_at` and deleting fired oneshots
//! up front), then each row's stored argv is dispatched through the
//! root `crate::run` — the same entry the binary and `plugins run`'s
//! nested-command path use — in parallel. Per-task streams are merged
//! via [`futures::stream::SelectAll`]; each item is wrapped with the
//! source schedule's `id` so callers can attribute output.
//!
//! Each task runs with the schedule's captured identity
//! (`apply_agent_arguments`) and the plugin that registered it
//! (`apply_plugin`) re-installed on the run ctx — both
//! `config.plugin_*` and `ctx.plugin`.
//!
//! Pre-stream `Err`s (e.g. the scheduled command failed to parse) are
//! re-emitted as a single-item error stream in the merged output.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::AgentArguments;
use objectiveai_sdk::cli::command::ResponseItem as RootResponseItem;
use objectiveai_sdk::cli::command::tasks::run::{Plugin, Request, ResponseItem};

use crate::context::Context;
use crate::db;
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, _request: Request) -> Result<ItemStream, Error> {
    // Scope is the caller's own AIH plus all descendants — no params.
    let parent = ctx.config.agent_instance_hierarchy.clone();

    let rows = db::tasks::collect_and_mark_pending(&ctx.db, &parent).await?;

    if rows.is_empty() {
        return Ok(Box::pin(futures::stream::empty()));
    }

    // Kick all per-task dispatches off in parallel. Each future
    // resolves to `(TaskMeta, Result<RootStream, Error>)` — the meta
    // is cloned out of the `RunRow` before `run_one` consumes it, and
    // tags every emitted item below. Pre-stream errors (parse /
    // handler-rejection) are folded into the merged stream as one-item
    // error streams.
    let starts = rows.into_iter().map(|row| {
        let ctx = ctx.clone();
        async move {
            let meta = TaskMeta {
                id: row.id,
                agent_instance_hierarchy: row.agent_instance_hierarchy.clone(),
                name: row.name.clone(),
                plugin: row.plugin.clone().map(|p| Plugin {
                    owner: p.owner,
                    repository: p.repository,
                    version: p.version,
                }),
            };
            let stream_result = run_one(&ctx, row).await;
            (meta, stream_result)
        }
    });
    let results = futures::future::join_all(starts).await;

    let mut select_all = futures::stream::SelectAll::new();
    for (meta, result) in results {
        match result {
            Ok(stream) => {
                let tagged = stream.map(move |r| {
                    r.map(|value| ResponseItem {
                        id: meta.id,
                        agent_instance_hierarchy: meta.agent_instance_hierarchy.clone(),
                        name: meta.name.clone(),
                        plugin: meta.plugin.clone(),
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

/// Envelope metadata for one fired schedule, cloned out of its
/// `RunRow` before `run_one` consumes the row, then stamped onto every
/// item that task emits.
struct TaskMeta {
    id: i64,
    agent_instance_hierarchy: String,
    name: String,
    plugin: Option<Plugin>,
}

/// Per-task stream — yields the typed root `ResponseItem` that
/// `crate::command::command::execute` already produces. No JSON
/// round-trip: the executor's `to_value` + `extract_leaf` dance
/// exists only to let the generic `execute<_, T>` settle on `T`,
/// and our `T` here *is* the root union, so we keep the value
/// typed end-to-end.
type RootStream = Pin<Box<dyn Stream<Item = Result<RootResponseItem, Error>> + Send>>;

/// Dispatch one stored schedule through the root `crate::run` — the
/// same entry `main.rs` and `plugins run`'s nested-command path use —
/// against a ctx carrying the schedule's captured identity and the
/// plugin that registered it. `crate::run` parses the argv itself, so
/// we prepend a placeholder program name (it strips `argv[0]`).
async fn run_one(ctx: &Context, row: db::tasks::RunRow) -> Result<RootStream, Error> {
    let mut task_ctx = apply_agent_arguments(ctx, &row.agent_arguments);
    apply_plugin(&mut task_ctx, row.plugin);

    let mut args = vec!["objectiveai-cli".to_string()];
    args.extend(row.command);
    crate::run(args, Some(task_ctx)).await
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

/// Re-install the schedule's registering plugin (if any) on the run
/// ctx — both `config.plugin_*` (so any subprocess the task spawns
/// inherits it via `apply_config_env`) and `ctx.plugin`. `None`
/// overrides whatever plugin the *caller* was running under, so a task
/// scheduled by a non-plugin never inherits one.
fn apply_plugin(ctx: &mut Context, plugin: Option<crate::plugin_path::PluginPath>) {
    match plugin {
        Some(p) => {
            ctx.config.plugin_owner = Some(p.owner.clone());
            ctx.config.plugin_repository = Some(p.repository.clone());
            ctx.config.plugin_version = Some(p.version.clone());
            ctx.plugin = Some(p);
        }
        None => {
            ctx.config.plugin_owner = None;
            ctx.config.plugin_repository = None;
            ctx.config.plugin_version = None;
            ctx.plugin = None;
        }
    }
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::tasks::run as sdk;
    use objectiveai_sdk::cli::command::tasks::run::request_schema::{
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
    use objectiveai_sdk::cli::command::tasks::run as sdk;
    use objectiveai_sdk::cli::command::tasks::run::response_schema::{
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
