//! `laboratories list` — stream the CONNECTED laboratories: the
//! daemon's `/laboratory` registry is the source of truth (a manager
//! that disconnects is no longer listed; podman is never consulted).
//! Read-only. Only client-side laboratories are supported today.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::laboratories::create::{EnvVar, Kind, Mount};
use objectiveai_sdk::cli::command::laboratories::list::{Request, ResponseItem};

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
    let stream = async_stream::stream! {
        match labs {
            Ok(labs) => {
                for lab in labs {
                    yield Ok(ResponseItem {
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
                    });
                }
            }
            Err(e) => yield Err(e),
        }
    };
    Ok(Box::pin(stream))
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
