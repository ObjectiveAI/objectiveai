//! `tools run` — resolve a tool by `(owner, name, version)`, build its
//! command from the current platform's exec vector + the caller's
//! args, run it with the tool's version folder as the working
//! directory, and yield each stdout/stderr line as a [`ResponseItem`]
//! as it arrives. A non-zero exit code surfaces as a final
//! `Err(Error::ToolExit(code))`.

use std::pin::Pin;
use std::process::Stdio;

use futures::Stream;
use objectiveai_sdk::cli::command::tools::run::{Request, ResponseItem};
use objectiveai_sdk::cli::{Error as CliError, ErrorType};
use tokio::process::Command;

use crate::child_io::{PipeEvent, spawn_pipe_reader};
use crate::context::Context;
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let coord = format!("{}/{}/{}", request.owner, request.name, request.version);
    let (exec, cwd) = ctx
        .filesystem
        .resolve_tool(&request.owner, &request.name, &request.version)
        .await
        .ok_or_else(|| Error::ToolNotFound(coord.clone()))?;

    // The command is the tool's exec vector merged with the caller's
    // args, verbatim — neither array's strings are inspected or
    // mutated. The first element is the program; the rest are its
    // arguments. CWD is the tool's version folder (where
    // `objectiveai.json` lives) — always.
    let mut argv = exec;
    argv.extend(request.args);
    let mut argv = argv.into_iter();
    let program = argv.next().ok_or_else(|| {
        Error::ToolNotFound(format!("{coord} (empty exec)"))
    })?;

    // Manifest exec paths are relative to the version folder (e.g.
    // `./count-tool.exe`), but on Windows `CreateProcess` resolves a
    // relative program against the PARENT's cwd, not the child's
    // `current_dir` (rust-lang/rust#37868) — the spawn would miss the
    // binary entirely. Absolutize relative *paths* against `cwd`;
    // bare names keep their PATH-lookup semantics (the POSIX execvp
    // rule: a separator means path, no separator means PATH search).
    //
    // Path-vs-name is decided by `Path::components()`, which encodes
    // the platform split for us:
    //   - Windows: `/` and `\` are both separators (and both illegal
    //     in file names), so either marks a path — 2+ components.
    //   - Unix: only `/` separates; `\` is a legal filename byte, so
    //     a program literally named `my\tool` stays a bare name —
    //     1 component — and still resolves via PATH.
    let program: std::ffi::OsString = {
        let path = std::path::Path::new(&program);
        if path.components().count() > 1 && path.is_relative() {
            cwd.join(path).into_os_string()
        } else {
            program.into()
        }
    };

    let mut cmd = Command::new(&program);
    cmd.args(argv)
        .current_dir(&cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    crate::spawn::apply_config_env(&mut cmd, &ctx.config);

    let mut child = cmd.spawn().map_err(Error::ToolSpawn)?;
    let stdout = child.stdout.take().expect("stdout was piped");
    let stderr = child.stderr.take().expect("stderr was piped");

    let mut events = spawn_pipe_reader(stdout, stderr);

    let stream = async_stream::stream! {
        while let Some(event) = events.recv().await {
            match event {
                PipeEvent::Stdout(line) => {
                    yield Ok(ResponseItem::Stdout(line));
                }
                PipeEvent::Stderr(line) => {
                    yield Ok(ResponseItem::Stderr(CliError {
                        r#type: ErrorType::Error,
                        level: None,
                        fatal: None,
                        message: serde_json::Value::String(line),
                    }));
                }
                PipeEvent::StdoutEof | PipeEvent::StderrEof => {}
                PipeEvent::StdoutErr(e) | PipeEvent::StderrErr(e) => {
                    yield Err(Error::ToolRead(e));
                    return;
                }
            }
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

pub mod request_schema {
    use objectiveai_sdk::cli::command::tools::run as sdk;
    use objectiveai_sdk::cli::command::tools::run::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::tools::run as sdk;
    use objectiveai_sdk::cli::command::tools::run::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::ResponseItem)))
    }
}
