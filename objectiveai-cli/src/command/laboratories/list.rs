//! `laboratories list` — stream the CONNECTED laboratories (the
//! daemon's `/laboratory` registry) FOLLOWED BY local laboratories
//! whose managers are not running, read back from podman by the
//! `objectiveai-laboratory list` subcommand (the CLI itself never
//! touches podman). `source` classifies by RAW id: anything the local
//! state-scoped scan knows is `local` (connected or not); anything
//! present only as a live connection is `remote` — including a
//! laboratory on this machine under a different state. Read-only.
//! Only client-side laboratories are supported today.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::laboratories::create::{EnvVar, Kind, Mount};
use objectiveai_sdk::cli::command::laboratories::list::{Request, ResponseItem, Source};

use crate::context::Context;
use crate::error::Error;
use objectiveai_sdk::client_objectiveai_mcp::laboratory::{SocketRequest, SocketResponse};

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    // Only `Client` exists today; the match stays exhaustive so adding
    // `Server` later forces a decision here.
    match request.kind {
        Kind::Client => {}
    }

    // The daemon owns the registry; ensure it's up (idempotent).
    crate::command::daemon::spawn::spawn(ctx).await?;
    let labs = match crate::websockets::websocket_laboratory::call_laboratories_socket(
        &ctx.filesystem.state_dir(),
        &SocketRequest::List,
    )
    .await
    {
        Ok(SocketResponse::List { laboratories }) => Ok(laboratories),
        Ok(SocketResponse::Error { message }) => Err(Error::Laboratory(message)),
        Ok(SocketResponse::Forwarded { .. }) => {
            Err(Error::Laboratory("unexpected socket reply".to_string()))
        }
        Err(e) => Err(Error::Laboratory(format!("laboratories socket: {e}"))),
    };
    // Local laboratories (running or not) via the manager binary's
    // `list` subcommand — the only podman reader left. A missing
    // binary means a remote-only install: the local set is empty.
    let local = match &labs {
        Ok(_) => local_laboratories(ctx).await?,
        Err(_) => Vec::new(),
    };
    let stream = async_stream::stream! {
        match labs {
            Ok(labs) => {
                let connected_ids: std::collections::HashSet<String> =
                    labs.iter().map(|l| l.id.clone()).collect();
                let local_ids: std::collections::HashSet<String> =
                    local.iter().map(|l| l.id.clone()).collect();
                for lab in labs {
                    let source = if local_ids.contains(&lab.id) {
                        Source::Local
                    } else {
                        Source::Remote
                    };
                    yield Ok(item_from_identify(lab, source));
                }
                for lab in local {
                    if !connected_ids.contains(&lab.id) {
                        yield Ok(item_from_identify(lab, Source::Local));
                    }
                }
            }
            Err(e) => yield Err(e),
        }
    };
    Ok(Box::pin(stream))
}

fn item_from_identify(
    lab: objectiveai_sdk::client_objectiveai_mcp::laboratory::Identify,
    source: Source,
) -> ResponseItem {
    ResponseItem {
        id: lab.id,
        image: lab.image,
        mounts: lab
            .mounts
            .into_iter()
            .map(|m| Mount {
                host: m.host,
                container: m.container,
            })
            .collect(),
        env: lab
            .env
            .into_iter()
            .map(|[key, value]| EnvVar { key, value })
            .collect(),
        cwd: lab.cwd,
        source,
    }
}

/// The local machine's laboratories (running or not), from the manager
/// binary's `list` subcommand. `Ok(vec![])` when the binary is not
/// installed (remote-only setups); `Err` when it exists but fails.
async fn local_laboratories(
    ctx: &Context,
) -> Result<Vec<objectiveai_sdk::client_objectiveai_mcp::laboratory::Identify>, Error> {
    let exe = ctx.filesystem.bin_dir().join(if cfg!(windows) {
        "objectiveai-laboratory.exe"
    } else {
        "objectiveai-laboratory"
    });
    let output = match tokio::process::Command::new(&exe)
        .arg("list")
        .arg("--objectiveai-dir")
        .arg(ctx.filesystem.dir())
        .arg("--objectiveai-state")
        .arg(ctx.filesystem.state())
        .output()
        .await
    {
        Ok(output) => output,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => {
            return Err(Error::Laboratory(format!(
                "spawn objectiveai-laboratory list: {e}"
            )));
        }
    };
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Laboratory(format!(
            "objectiveai-laboratory list: {}",
            stderr.trim()
        )));
    }
    serde_json::from_slice(&output.stdout).map_err(|e| {
        Error::Laboratory(format!("parse objectiveai-laboratory list output: {e}"))
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::laboratories::list as sdk;
    use objectiveai_sdk::cli::command::laboratories::list::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Request),
        ))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::laboratories::list as sdk;
    use objectiveai_sdk::cli::command::laboratories::list::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::ResponseItem),
        ))
    }
}
