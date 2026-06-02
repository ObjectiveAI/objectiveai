//! `tools run` — bare-naked port of the legacy `dispatch_tool`.
//!
//! Resolve the installed tool binary via
//! [`crate::filesystem::Client::resolve_tool`], spawn it with the
//! caller-supplied args, drain stdout/stderr line-buffered, and yield
//! each line as a [`ResponseItem`]. A non-zero exit code is yielded
//! as a trailing `Stderr` item with `fatal: true` — the bare-naked
//! contract surfaces exit state through the wire shape rather than
//! a fatal `Err`.

use std::pin::Pin;
use std::process::Stdio;

use futures::Stream;
use objectiveai_sdk::cli::command::tools::run::{Request, ResponseItem};
use objectiveai_sdk::cli::{Error as CliError, ErrorType, Level};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::context::Context;
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let exe = ctx
        .filesystem
        .resolve_tool(&request.name)
        .await
        .ok_or_else(|| Error::ToolNotFound(request.name.clone()))?;

    let mut cmd = Command::new(&exe);
    cmd.args(&request.args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    crate::spawn::apply_config_env(&mut cmd, &ctx.config);

    let mut child = cmd.spawn().map_err(Error::ToolSpawn)?;
    let stdout = child.stdout.take().expect("stdout was piped");
    let stderr = child.stderr.take().expect("stderr was piped");

    let (tx, rx) = mpsc::channel::<Result<ResponseItem, Error>>(16);
    tokio::spawn(async move {
        tokio::join!(
            drain_stdout(stdout, tx.clone()),
            drain_stderr(stderr, tx.clone()),
        );
        match child.wait().await {
            Ok(status) if status.success() => {}
            Ok(status) => {
                let code = status.code().unwrap_or(1);
                let _ = tx
                    .send(Ok(ResponseItem::Stderr(cli_error(
                        true,
                        format!("tool exited with code {code}"),
                    ))))
                    .await;
            }
            Err(e) => {
                let _ = tx.send(Err(Error::ToolRead(e))).await;
            }
        }
    });
    Ok(Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx)))
}

async fn drain_stdout<R>(stream: R, tx: mpsc::Sender<Result<ResponseItem, Error>>)
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) | Err(_) => return,
            Ok(_) => {
                let trimmed = line.trim_end_matches(['\r', '\n']).to_string();
                if tx.send(Ok(ResponseItem::Stdout(trimmed))).await.is_err() {
                    return;
                }
            }
        }
    }
}

async fn drain_stderr<R>(stream: R, tx: mpsc::Sender<Result<ResponseItem, Error>>)
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) | Err(_) => return,
            Ok(_) => {
                let trimmed = line.trim_end_matches(['\r', '\n']).to_string();
                let item = ResponseItem::Stderr(cli_error(false, trimmed));
                if tx.send(Ok(item)).await.is_err() {
                    return;
                }
            }
        }
    }
}

fn cli_error(fatal: bool, message: String) -> CliError {
    CliError {
        r#type: ErrorType::Error,
        level: Some(Level::Error),
        fatal: Some(fatal),
        message: serde_json::Value::String(message),
    }
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::tools::run as sdk;
    use objectiveai_sdk::cli::command::tools::run::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::tools::run as sdk;
    use objectiveai_sdk::cli::command::tools::run::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
