//! `agents tasks run` — fire every pending schedule in the caller's
//! own subtree, recording a run row and a full output log per firing.
//!
//! Scope is fixed to the caller's own AIH
//! (`ctx.config.agent_instance_hierarchy`) plus every descendant.
//! `db::tasks::claim_pending` atomically selects the pending rows AND
//! inserts their `tasks_runs` rows (advisory-locked, so concurrent
//! `tasks run` callers never double-fire a schedule); each claimed
//! row's stored argv is then dispatched through the root `crate::run`
//! — the same entry the binary and `plugins run`'s nested-command path
//! use — in parallel. Per-task streams are merged via
//! [`futures::stream::SelectAll`]; each item is wrapped with the
//! source schedule's identity (`name` / `aih` / `version` / `plugin`).
//!
//! A log-writer listener is spawned at the top of `execute` with an
//! unbounded receiver; the log wrapper (layer 1, identical in both
//! modes) sends every envelope (with its claim's `run_id`, which rides
//! next to the item internally and never hits the wire) into the
//! channel, and the listener appends each to `tasks_logs` as it
//! arrives. After the merge is exhausted the wrapper drops the sender
//! and awaits the listener so every log line is durably written before
//! the stream ends.
//!
//! Output has two modes (layer 2), selected by `request.stream_all`:
//! `--stream-all` passes every envelope through verbatim
//! (`ResponseItem::Value`); the default engages the success layer
//! instead — items are swallowed and each task yields exactly one
//! `ResponseItem::Success { .., success }` when its stream completes,
//! `success` being `false` iff the task's final item was an error.
//! The mode never affects what is logged.
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
use std::collections::HashMap;

use objectiveai_sdk::cli::command::tasks::run::{
    Plugin, Request, ResponseItem, RunValue, SuccessResponseItem, ValueResponseItem,
};

use crate::RunStream;
use crate::context::Context;
use crate::db;
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

/// Internal merge event. Items are tagged with their claim's `run_id`
/// so the log wrapper can hand every line to the log writer — the id
/// rides next to the item and never appears on the wire. `Done` marks
/// a task's stream completing (carrying the meta the success layer
/// needs); `WriterFailed` carries the log-writer finalizer's error
/// through the layers.
enum TaskEvent {
    Item(i64, Result<ResponseItem, Error>),
    Done(TaskMeta),
    WriterFailed(Error),
}

type TaggedStream = Pin<Box<dyn Stream<Item = TaskEvent> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    // The log-writer listener: spawned up front with an unbounded
    // receiver. It appends each (run_id, line) to `tasks_logs` as they
    // come in, and exits once every sender has dropped.
    let (log_tx, log_rx) = tokio::sync::mpsc::unbounded_channel::<(i64, String)>();
    let writer = tokio::spawn(log_writer_loop(ctx.db_client().await?.clone(), log_rx));

    // Scope is the caller's own AIH plus all descendants — no params.
    let parent = ctx.config.agent_instance_hierarchy.clone();

    let rows = db::tasks::claim_pending(ctx.db_client().await?, &parent).await?;

    if rows.is_empty() {
        // Nothing claimed → nothing to log. `log_tx` drops here and
        // the writer exits on its own.
        return Ok(Box::pin(futures::stream::empty()));
    }

    // Kick all per-task dispatches off in parallel. Each future
    // resolves to `(TaskMeta, Result<RunStream, Error>)` — the meta
    // is cloned out of the `RunRow` before `run_one` consumes it, and
    // tags every emitted item below. Pre-stream errors (parse /
    // handler-rejection) are folded into the merged stream as one-item
    // error streams.
    let starts = rows.into_iter().map(|row| {
        let ctx = ctx.clone();
        async move {
            let meta = TaskMeta {
                run_id: row.run_id,
                agent_instance_hierarchy: row.agent_instance_hierarchy.clone(),
                name: row.name.clone(),
                version: row.version as u64,
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
        let run_id = meta.run_id;
        // Each task's stream ends with a `Done` marker carrying its
        // meta, so the success layer can tell when a task completed.
        let done = meta.clone();
        match result {
            // Typed (no transform): wrap each root item as an
            // `ExecuteValue`.
            Ok(RunStream::Execute(stream)) => {
                let tagged = stream
                    .map(move |r| {
                        TaskEvent::Item(
                            run_id,
                            r.map(|value| {
                                ResponseItem::Value(ValueResponseItem {
                                    agent_instance_hierarchy: meta
                                        .agent_instance_hierarchy
                                        .clone(),
                                    name: meta.name.clone(),
                                    version: meta.version,
                                    plugin: meta.plugin.clone(),
                                    value: RunValue::ExecuteValue(Box::new(value)),
                                })
                            }),
                        )
                    })
                    .chain(futures::stream::once(async move {
                        TaskEvent::Done(done)
                    }));
                select_all.push(Box::pin(tagged) as TaggedStream);
            }
            // Transformed: wrap each post-transform JSON as an
            // `ExecuteTransformValue`.
            Ok(RunStream::ExecuteTransform(stream)) => {
                let tagged = stream
                    .map(move |r| {
                        TaskEvent::Item(
                            run_id,
                            r.map(|output| {
                                ResponseItem::Value(ValueResponseItem {
                                    agent_instance_hierarchy: meta
                                        .agent_instance_hierarchy
                                        .clone(),
                                    name: meta.name.clone(),
                                    version: meta.version,
                                    plugin: meta.plugin.clone(),
                                    value: RunValue::ExecuteTransformValue(output),
                                })
                            }),
                        )
                    })
                    .chain(futures::stream::once(async move {
                        TaskEvent::Done(done)
                    }));
                select_all.push(Box::pin(tagged) as TaggedStream);
            }
            Err(e) => {
                let tagged = futures::stream::once(async move {
                    TaskEvent::Item(run_id, Err(e))
                })
                .chain(futures::stream::once(async move {
                    TaskEvent::Done(done)
                }));
                select_all.push(Box::pin(tagged) as TaggedStream);
            }
        }
    }

    // Layer 1 — the log wrapper, identical regardless of mode: send
    // each Ok envelope (serialized, keyed by its run_id) to the log
    // writer and pass every event through. Once the merge is
    // exhausted, drop the sender and wait for the listener to finish
    // draining its queue so every line is durably in `tasks_logs`
    // before the stream ends; a writer failure flows on as a trailing
    // event.
    let logged: TaggedStream = Box::pin(async_stream::stream! {
        let mut merged = select_all;
        while let Some(event) = merged.next().await {
            if let TaskEvent::Item(run_id, Ok(envelope)) = &event {
                if let Ok(line) = serde_json::to_string(envelope) {
                    // A dead writer (an earlier insert failed) must
                    // not stop the user-facing stream — its error
                    // surfaces from the join below.
                    let _ = log_tx.send((*run_id, line));
                }
            }
            yield event;
        }
        drop(log_tx);
        match writer.await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => yield TaskEvent::WriterFailed(e),
            Err(_) => yield TaskEvent::WriterFailed(Error::WriterPanic),
        }
    });

    // Layer 2 — the mode adapter. `--stream-all` skips the success
    // layer entirely: items pass through verbatim and the Done markers
    // vanish. The default engages the success layer: items are
    // swallowed (each run's final-item error-ness tracked) and every
    // task yields exactly one `{.., success}` summary on its Done.
    let stream: ItemStream = if request.stream_all {
        Box::pin(logged.filter_map(|event| async move {
            match event {
                TaskEvent::Item(_, item) => Some(item),
                TaskEvent::Done(_) => None,
                TaskEvent::WriterFailed(e) => Some(Err(e)),
            }
        }))
    } else {
        Box::pin(async_stream::stream! {
            let mut last_err: HashMap<i64, bool> = HashMap::new();
            let mut inner = logged;
            while let Some(event) = inner.next().await {
                match event {
                    TaskEvent::Item(run_id, item) => {
                        last_err.insert(run_id, item.is_err());
                    }
                    TaskEvent::Done(meta) => {
                        let success =
                            !last_err.get(&meta.run_id).copied().unwrap_or(false);
                        yield Ok(ResponseItem::Success(SuccessResponseItem {
                            agent_instance_hierarchy: meta.agent_instance_hierarchy,
                            name: meta.name,
                            version: meta.version,
                            plugin: meta.plugin,
                            success,
                        }));
                    }
                    TaskEvent::WriterFailed(e) => yield Err(e),
                }
            }
        })
    };

    Ok(stream)
}

/// Drain the log channel into `tasks_logs` — one INSERT per emitted
/// item, written as they come in. Returns once every sender has
/// dropped and the queue is empty; a failed INSERT exits early and the
/// error surfaces as the run stream's trailing item.
async fn log_writer_loop(
    pool: crate::db::Pool,
    mut rx: tokio::sync::mpsc::UnboundedReceiver<(i64, String)>,
) -> Result<(), Error> {
    while let Some((run_id, value)) = rx.recv().await {
        db::tasks::insert_task_log(&pool, run_id, &value).await?;
    }
    Ok(())
}

/// Envelope metadata for one fired schedule, cloned out of its
/// `RunRow` before `run_one` consumes the row, then stamped onto every
/// item that task emits (and onto its terminal `Done` marker, which
/// the success layer turns into the per-task summary). `run_id` tags
/// items internally for the log writer and never appears on the wire
/// envelope.
#[derive(Clone)]
struct TaskMeta {
    run_id: i64,
    agent_instance_hierarchy: String,
    name: String,
    version: u64,
    plugin: Option<Plugin>,
}

/// Dispatch one stored schedule through the root `crate::run` — the
/// same entry `main.rs` and `plugins run`'s nested-command path use —
/// against a ctx carrying the schedule's captured identity and the
/// plugin that registered it. `crate::run` parses the argv itself, so
/// we prepend a placeholder program name (it strips `argv[0]`). The
/// returned [`RunStream`] tells us whether the scheduled command was
/// typed or transformed; the caller tags each item accordingly.
async fn run_one(ctx: &Context, row: db::tasks::RunRow) -> Result<RunStream, Error> {
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
/// `agent_instance_hierarchy` field and the API-client cell detach
/// (the memoized `HttpClient` must rebuild with the task's identity
/// headers).
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
    ctx.reset_api_client();
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
