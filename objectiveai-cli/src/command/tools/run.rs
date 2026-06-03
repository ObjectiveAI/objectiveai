//! `tools run` — bare-naked port of the legacy `dispatch_tool`.
//!
//! Resolve the installed tool binary via
//! [`crate::filesystem::Client::resolve_tool`], spawn it with the
//! caller-supplied args, and yield each stdout/stderr line as a
//! [`ResponseItem`] as it arrives — driven directly by the caller
//! polling the returned stream, the same way the legacy
//! `dispatch_tool` emits each line inline. A non-zero exit code
//! surfaces as a final `Err(Error::ToolExit(code))`, matching
//! legacy behavior.

use std::pin::Pin;
use std::process::Stdio;

use futures::Stream;
use futures::StreamExt;
use objectiveai_sdk::cli::command::tools::run::{Request, ResponseItem};
use objectiveai_sdk::cli::{Error as CliError, ErrorType};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio_stream::StreamExt as _;
use tokio_stream::wrappers::LinesStream;

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

    let stdout_lines = StreamExt::map(
        LinesStream::new(BufReader::new(stdout).lines()),
        |r: Result<String, std::io::Error>| {
            r.map(stdout_item).map_err(Error::ToolRead)
        },
    );
    let stderr_lines = StreamExt::map(
        LinesStream::new(BufReader::new(stderr).lines()),
        |r: Result<String, std::io::Error>| {
            r.map(stderr_item).map_err(Error::ToolRead)
        },
    );

    // `merge` polls both line streams and yields each item the
    // moment it's read — no buffering between the producer and the
    // caller polling this stream. Mirrors the legacy `tokio::join!`
    // of two `forward_stream` futures, except items flow through the
    // stream's poll path instead of `Handle::emit`.
    let merged = stdout_lines.merge(stderr_lines);

    let stream = async_stream::stream! {
        let mut merged = merged;
        while let Some(item) = futures::StreamExt::next(&mut merged).await {
            yield item;
        }
        // Both pipes closed — child has either exited or is about to.
        // Wait for the exit code and surface non-zero as a stream
        // `Err`, the same way legacy `dispatch_tool` returns
        // `Err(Error::ToolExit(code))`.
        match child.wait().await {
            Ok(status) if status.success() => {}
            Ok(status) => {
                yield Err(Error::ToolExit(status.code().unwrap_or(1)));
            }
            Err(e) => yield Err(Error::ToolRead(e)),
        }
    };

    Ok(Box::pin(stream))
}

fn stdout_item(line: String) -> ResponseItem {
    ResponseItem::Stdout(trim_eol(line))
}

fn stderr_item(line: String) -> ResponseItem {
    ResponseItem::Stderr(CliError {
        r#type: ErrorType::Error,
        level: None,
        fatal: None,
        message: serde_json::Value::String(trim_eol(line)),
    })
}

fn trim_eol(mut line: String) -> String {
    while matches!(line.chars().last(), Some('\r') | Some('\n')) {
        line.pop();
    }
    line
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
        Ok(schemars::schema_for!(sdk::ResponseItem))
    }
}
