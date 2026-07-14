//! The resident daemon's `/execute` route — the server side of
//! [`objectiveai_sdk::cli::command::SseCommandExecutor`].
//!
//! Request-per-command over plain HTTP: the client POSTs the SDK
//! [`ExecuteEnvelope`] (a `cli::command::Request` as serde JSON plus an
//! optional [`AgentArguments`] identity override) as the request body,
//! authenticating with the `X-OBJECTIVEAI-SIGNATURE` header (verified by
//! [`crate::websockets::daemon_auth::authenticate_header`]). The daemon
//! streams the result back as Server-Sent Events.
//!
//! The daemon runs the request IN-PROCESS via the re-entrant
//! [`crate::run`] (the same path `plugins run` uses for nested plugin
//! commands) against a clone of its resident [`Context`] with the
//! override applied ([`crate::executor::apply_agent_arguments`] —
//! `mcp_session_id` is ignored: a remote caller has no business joining
//! the daemon's MCP sessions, and the daemon's own slot is scrubbed at
//! spawn). The daemon's filesystem layout and secret are never
//! overridable.
//!
//! Each stream item goes back as one SSE `data:` event in exactly the
//! cli's stdout JSONL line shapes (`main.rs::drain`): `Ok` items as
//! their JSON, `Err` items as the structured
//! [`objectiveai_sdk::cli::Error`] line. When the stream ends the
//! response body closes — that close IS the end-of-stream marker (the
//! HTTP equivalent of the old WebSocket `Close`). A client disconnect
//! mid-stream aborts the request, which drops the in-process run stream
//! and cancels the command.
//!
//! Because the run re-enters [`crate::run`], the producer tee applies:
//! `/execute` runs are broadcast on `/listen` like any other CLI
//! activity, with the overridden identity in the request frame's
//! context. `/execute` streams never carry broadcast data and `/listen`
//! never accepts requests.

use axum::response::sse::{Event, KeepAlive, Sse};
use futures::StreamExt;
use objectiveai_sdk::cli::command::command_executor::sse::ExecuteEnvelope;

use crate::context::Context;
use crate::error::Error;

/// `POST /execute`: header-auth, then run one command in-process and
/// stream its items back as SSE.
pub(crate) async fn execute_handler(
    axum::extract::State(state): axum::extract::State<
        crate::websockets::daemon_stream::DaemonWsState,
    >,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    if !crate::websockets::daemon_auth::authenticate_header(&headers, state.secret.as_ref()) {
        return axum::http::StatusCode::UNAUTHORIZED.into_response();
    }
    Sse::new(execute_stream(state.ctx, body))
        .keep_alive(KeepAlive::default())
        .into_response()
}

/// Decode the envelope, run in-process, and yield each item as one SSE
/// event. Envelope/run errors ride in-band as `cli::Error` events (the
/// same shape a mid-stream error uses), then the stream ends. The body
/// ending is the end-of-stream marker.
fn execute_stream(
    ctx: Context,
    body: axum::body::Bytes,
) -> impl futures::Stream<Item = Result<Event, std::convert::Infallible>> {
    async_stream::stream! {
        let envelope: ExecuteEnvelope = match serde_json::from_slice(&body) {
            Ok(envelope) => envelope,
            Err(e) => {
                yield Ok(error_event(format!("decode execute envelope: {e}")));
                return;
            }
        };

        // Per-request identity override — sans `mcp_session_id`, which is
        // stripped so `apply_agent_arguments` clears rather than adopts it.
        let agent_arguments = envelope.agent_arguments.map(|mut args| {
            args.mcp_session_id = None;
            args
        });
        let ctx = crate::executor::apply_agent_arguments(&ctx, agent_arguments.as_ref())
            .into_owned();

        // The same re-entry `plugins run` uses for nested commands:
        // `crate::run` strips args[0] unconditionally, so prepend a
        // placeholder and dispatch the request JSON through the top-level
        // `--request` front door.
        let request_json = match serde_json::to_string(&envelope.request) {
            Ok(json) => json,
            Err(e) => {
                yield Ok(error_event(format!("serialize execute request: {e}")));
                return;
            }
        };
        let args = vec![
            "objectiveai".to_string(),
            "--request".to_string(),
            request_json,
        ];
        match crate::run(args, Some(ctx)).await {
            Ok(crate::RunStream::Execute(mut stream)) => {
                while let Some(item) = stream.next().await {
                    yield Ok(item_event(item));
                }
            }
            Ok(crate::RunStream::ExecuteTransform(mut stream)) => {
                while let Some(item) = stream.next().await {
                    yield Ok(item_event(item));
                }
            }
            Err(e) => {
                yield Ok(error_event_from(&e));
            }
        }
    }
}

/// One stream item → one SSE event, mirroring `main.rs::drain` /
/// `write_line`: `Ok` values as their JSON, `Err` values as the
/// structured `cli::Error` line.
fn item_event<T: serde::Serialize>(item: Result<T, Error>) -> Event {
    let line = match item {
        Ok(value) => serde_json::to_string(&value).unwrap_or_else(|e| {
            error_line(serde_json::Value::String(format!("serialize error: {e}")))
        }),
        Err(e) => error_line(e.output_message()),
    };
    Event::default().data(line)
}

/// An SSE event carrying a structured `cli::Error` line from a plain
/// message.
fn error_event(message: String) -> Event {
    Event::default().data(error_line(serde_json::Value::String(message)))
}

/// An SSE event carrying a structured `cli::Error` line from a daemon
/// [`Error`].
fn error_event_from(e: &Error) -> Event {
    Event::default().data(error_line(e.output_message()))
}

/// The `main.rs::write_error_line` JSONL shape as a string.
fn error_line(message: serde_json::Value) -> String {
    let payload = objectiveai_sdk::cli::Error {
        r#type: objectiveai_sdk::cli::ErrorType::Error,
        level: Some(objectiveai_sdk::cli::Level::Error),
        fatal: None,
        message,
    };
    serde_json::to_string(&payload).unwrap_or_else(|_| {
        r#"{"type":"error","fatal":false,"message":"serialize error"}"#.to_string()
    })
}
