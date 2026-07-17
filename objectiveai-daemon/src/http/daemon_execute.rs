//! The resident daemon's `/execute` route — the server side of
//! [`objectiveai_sdk::cli::command::SseCommandExecutor`].
//!
//! Request-per-command over plain HTTP: the client POSTs the
//! `cli::command::Request` serde JSON as the raw request body — nothing
//! wraps it — authenticating with the `X-OBJECTIVEAI-SIGNATURE` header
//! (verified by [`crate::http::daemon_auth::authenticate_header`]).
//! The daemon streams the result back as Server-Sent Events.
//!
//! The [`AgentArguments`] identity rides the
//! [`AGENT_ARGUMENT_HEADERS`] request headers (the same
//! `X-OBJECTIVEAI-*` names the api stamps on outbound calls), one
//! header per field. A missing header DELETES that config field for
//! the run — the daemon never inherits its own resident value.
//!
//! The daemon runs the request IN-PROCESS via the re-entrant
//! [`crate::run`] (the same path `plugins run` uses for nested plugin
//! commands) against its resident context pair with the
//! override applied ([`crate::executor::apply_agent_arguments`]).
//! The daemon's filesystem layout and secret are never overridable.
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

use axum::response::sse::{Event, Sse};
use futures::StreamExt;
use objectiveai_sdk::cli::command::AgentArguments;

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

/// `POST /execute`: header-auth, then run one command in-process and
/// stream its items back as SSE.
pub(crate) async fn execute_handler(
    axum::extract::State(state): axum::extract::State<
        crate::http::daemon_stream::DaemonHttpState,
    >,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    if !crate::http::daemon_auth::authenticate_header(&headers, state.secret.as_ref()) {
        return axum::http::StatusCode::UNAUTHORIZED.into_response();
    }
    Sse::new(execute_stream(
        state.global,
        state.scoped,
        agent_arguments(&headers),
        body,
    ))
    .into_response()
}

/// The per-request identity from the `X-OBJECTIVEAI-*` request headers
/// — the same names the api stamps on outbound calls, one header per
/// field. A missing (or non-UTF-8) header is `None`, which
/// [`crate::executor::apply_agent_arguments`] DELETES on the run's
/// scope — never inherits.
fn agent_arguments(headers: &axum::http::HeaderMap) -> AgentArguments {
    let get = |name: &str| {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(String::from)
    };
    AgentArguments {
        agent_instance_hierarchy: get("X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY"),
        agent_id: get("X-OBJECTIVEAI-AGENT-ID"),
        agent_full_id: get("X-OBJECTIVEAI-AGENT-FULL-ID"),
        agent_remote: get("X-OBJECTIVEAI-AGENT-REMOTE"),
        response_id: get("X-OBJECTIVEAI-RESPONSE-ID"),
        response_ids: get("X-OBJECTIVEAI-RESPONSE-IDS"),
    }
}

/// Run the raw-body request in-process and yield each item as one SSE
/// event. Errors ride in-band as `cli::Error` events (the same shape a
/// mid-stream error uses), then the stream ends. The body ending is the
/// end-of-stream marker.
fn execute_stream(
    global: GlobalContext,
    scoped: ScopedContext,
    agent_arguments: AgentArguments,
    body: axum::body::Bytes,
) -> impl futures::Stream<Item = Result<Event, std::convert::Infallible>> {
    async_stream::stream! {
        let scoped =
            crate::executor::apply_agent_arguments(&scoped, Some(&agent_arguments))
                .await
                .into_owned();

        // The same re-entry `plugins run` uses for nested commands:
        // `crate::run` strips args[0] unconditionally, so prepend a
        // placeholder and dispatch the raw body — the request JSON —
        // through the top-level `--request` front door.
        let request_json = match String::from_utf8(body.to_vec()) {
            Ok(json) => json,
            Err(e) => {
                yield Ok(error_event(format!("decode execute request: {e}")));
                return;
            }
        };
        let args = vec![
            "objectiveai".to_string(),
            "--request".to_string(),
            request_json,
        ];
        match crate::run(args, Some((global, scoped))).await {
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
