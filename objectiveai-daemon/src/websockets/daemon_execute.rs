//! The resident daemon's `/execute` WebSocket route — the server side
//! of [`objectiveai_sdk::cli::command::WebSocketExecutor`].
//!
//! Connection-per-command: the client sends the auth preamble (the
//! SDK `AuthEnvelope`, verified by
//! [`crate::websockets::daemon_auth::authenticate`]), then ONE text
//! message (the SDK [`ExecuteEnvelope`] — a `cli::command::Request` as
//! serde JSON plus an optional [`AgentArguments`] identity override),
//! then only reads.
//! The daemon runs the request IN-PROCESS via the re-entrant
//! [`crate::run`] (the same path `plugins run` uses for nested plugin
//! commands) against a clone of its resident [`Context`] with the
//! override applied ([`crate::executor::apply_agent_arguments`] —
//! `mcp_session_id` is ignored: a remote caller has no business
//! joining the daemon's MCP sessions, and the daemon's own slot is
//! scrubbed at spawn). The daemon's filesystem layout and secret are
//! never overridable.
//!
//! Each stream item goes back as one text message in exactly the cli's
//! stdout JSONL line shapes (`main.rs::drain`): `Ok` items as their
//! JSON, `Err` items as the structured [`objectiveai_sdk::cli::Error`]
//! line. When the stream ends the daemon closes the socket — that
//! close IS the end-of-stream marker. A client disconnect mid-stream
//! fails the next send, which drops the in-process run stream and
//! cancels the command.
//!
//! Because the run re-enters [`crate::run`], the producer tee applies:
//! `/execute` runs are broadcast on `/listen` like any other CLI
//! activity, with the overridden identity in the request frame's
//! context. `/execute` streams never carry broadcast data and
//! `/listen` never accepts requests.

use axum::extract::ws::{Message, WebSocket};
use futures::StreamExt;
use objectiveai_sdk::cli::command::command_executor::websocket::ExecuteEnvelope;

use crate::context::Context;
use crate::error::Error;

/// `/execute`: upgrade, consume the auth preamble, and run one
/// command over the socket.
pub(crate) async fn execute_handler(
    axum::extract::State(state): axum::extract::State<
        crate::websockets::daemon_stream::DaemonWsState,
    >,
    ws: axum::extract::ws::WebSocketUpgrade,
) -> axum::response::Response {
    ws.on_upgrade(move |mut socket| async move {
        if !crate::websockets::daemon_auth::authenticate(&mut socket, state.secret.as_ref()).await
        {
            return;
        }
        handle_execute(socket, state.ctx).await;
    })
}

/// Drive one command: read the envelope, run in-process, stream the
/// items back, close. The auth preamble has already been consumed.
async fn handle_execute(mut socket: WebSocket, ctx: Context) {
    // The next text message is the envelope; anything else before it
    // (ping/pong/binary) is ignored. Client gone before sending → done.
    let envelope = loop {
        match socket.recv().await {
            Some(Ok(Message::Text(text))) => break text,
            Some(Ok(Message::Close(_))) | Some(Err(_)) | None => return,
            Some(Ok(_)) => continue,
        }
    };
    let envelope: ExecuteEnvelope = match serde_json::from_str(&envelope) {
        Ok(envelope) => envelope,
        Err(e) => {
            send_error_line(
                &mut socket,
                serde_json::Value::String(format!("decode execute envelope: {e}")),
            )
            .await;
            let _ = socket.send(Message::Close(None)).await;
            return;
        }
    };

    // Per-request identity override — sans `mcp_session_id`, which is
    // stripped so `apply_agent_arguments` clears rather than adopts it.
    let agent_arguments = envelope.agent_arguments.map(|mut args| {
        args.mcp_session_id = None;
        args
    });
    let ctx = crate::executor::apply_agent_arguments(&ctx, agent_arguments.as_ref()).into_owned();

    // The same re-entry `plugins run` uses for nested commands:
    // `crate::run` strips args[0] unconditionally, so prepend a
    // placeholder and dispatch the request JSON through the top-level
    // `--request` front door.
    let request_json = match serde_json::to_string(&envelope.request) {
        Ok(json) => json,
        Err(e) => {
            send_error_line(
                &mut socket,
                serde_json::Value::String(format!("serialize execute request: {e}")),
            )
            .await;
            let _ = socket.send(Message::Close(None)).await;
            return;
        }
    };
    let args = vec![
        "objectiveai".to_string(),
        "--request".to_string(),
        request_json,
    ];
    match crate::run(args, Some(ctx)).await {
        Ok(crate::RunStream::Execute(stream)) => drain(&mut socket, stream).await,
        Ok(crate::RunStream::ExecuteTransform(stream)) => drain(&mut socket, stream).await,
        Err(e) => {
            let _ = send_error(&mut socket, &e).await;
        }
    }
    let _ = socket.send(Message::Close(None)).await;
}

/// Forward every stream item to the client — a mirror of
/// `main.rs::drain`, with `ws.send` in place of stdout. A failed send
/// means the client is gone: return, dropping the stream (which
/// cancels the in-process run).
async fn drain<S, T>(socket: &mut WebSocket, mut stream: S)
where
    S: futures::Stream<Item = Result<T, Error>> + Unpin,
    T: serde::Serialize,
{
    while let Some(item) = stream.next().await {
        let sent = match item {
            Ok(value) => send_line(socket, &value).await,
            Err(e) => send_error(socket, &e).await,
        };
        if !sent {
            return;
        }
    }
}

/// One JSONL frame — `main.rs::write_line` over the socket. Returns
/// whether the send succeeded.
async fn send_line<T: serde::Serialize>(socket: &mut WebSocket, value: &T) -> bool {
    let line = match serde_json::to_string(value) {
        Ok(s) => s,
        Err(e) => format!(r#"{{"type":"error","fatal":false,"message":"serialize error: {e}"}}"#),
    };
    socket.send(Message::Text(line.into())).await.is_ok()
}

/// One structured error frame — `main.rs::write_error_line` over the
/// socket. Returns whether the send succeeded.
async fn send_error(socket: &mut WebSocket, e: &Error) -> bool {
    send_error_line(socket, e.output_message()).await
}

async fn send_error_line(socket: &mut WebSocket, message: serde_json::Value) -> bool {
    let payload = objectiveai_sdk::cli::Error {
        r#type: objectiveai_sdk::cli::ErrorType::Error,
        level: Some(objectiveai_sdk::cli::Level::Error),
        fatal: None,
        message,
    };
    send_line(socket, &payload).await
}
