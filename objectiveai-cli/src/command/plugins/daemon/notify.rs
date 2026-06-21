//! `plugins daemon notify` — deliver one input to a resident daemon
//! plugin's stdin over its socket.
//!
//! Ensures the per-state plugin daemon is up, connects to the target
//! plugin's socket, writes `input` as one JSON line, and returns the
//! daemon's ack. Like `plugins run`'s plugin-can't-run-plugins guard, a
//! plugin may only notify ITSELF — never another plugin.

use std::path::Path;

use objectiveai_sdk::cli::command::plugins::daemon::notify::{Request, Response};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::command::daemon::socket;
use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    // Self-only: a plugin may only `daemon notify` itself, mirroring
    // the plugin-can't-run-plugins guard in `plugins run`.
    if let Some(plugin) = ctx.plugin.as_ref()
        && (plugin.owner != request.owner
            || plugin.repository != request.name
            || plugin.version != request.version)
    {
        return Err(Error::DaemonNotifyNotSelf {
            owner: request.owner,
            name: request.name,
            version: request.version,
        });
    }

    crate::command::daemon::spawn::spawn(ctx).await?;

    let socket_path = socket::plugin_socket_path(
        &ctx.filesystem.state_dir(),
        &request.owner,
        &request.name,
        &request.version,
    );
    let conn = connect_with_retry(&socket_path).await.map_err(|e| {
        Error::Daemon(format!(
            "could not connect to daemon socket for {}/{}/{} ({e}); is `daemon = true` set in its manifest?",
            request.owner, request.name, request.version,
        ))
    })?;

    // Send `input` as one JSON line; the daemon forwards it to the
    // plugin's stdin and acks.
    let line = serde_json::to_string(&request.input).map_err(Error::InlineJson)?;
    let (read_half, mut write_half) = tokio::io::split(conn);
    let send = async {
        write_half.write_all(line.as_bytes()).await?;
        write_half.write_all(b"\n").await?;
        write_half.flush().await
    };
    send.await.map_err(|e| Error::Daemon(format!("send: {e}")))?;

    let mut reader = BufReader::new(read_half);
    let mut ack = String::new();
    reader
        .read_line(&mut ack)
        .await
        .map_err(|e| Error::Daemon(format!("read ack: {e}")))?;
    let response: Response =
        serde_json::from_str(ack.trim()).unwrap_or(Response { ok: false });
    Ok(response)
}

/// Connect to a freshly-bound daemon socket, retrying briefly to cover
/// the OS-level availability lag right after the daemon binds (the
/// daemon lock is only published after every socket is bound, so this
/// almost always succeeds on the first try).
async fn connect_with_retry(socket_path: &Path) -> std::io::Result<socket::Stream> {
    let mut last: Option<std::io::Error> = None;
    for _ in 0..50 {
        match socket::connect(socket_path).await {
            Ok(stream) => return Ok(stream),
            Err(e) => {
                last = Some(e);
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }
    }
    Err(last.unwrap_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "socket not available")
    }))
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::plugins::daemon::notify as sdk;
    use objectiveai_sdk::cli::command::plugins::daemon::notify::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::plugins::daemon::notify as sdk;
    use objectiveai_sdk::cli::command::plugins::daemon::notify::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
