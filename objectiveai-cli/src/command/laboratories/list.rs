//! `laboratories list` — stream the laboratory containers created in this
//! state by reading them back from podman (the source of truth). Read-only.
//! Only client-side laboratories are supported today.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::laboratories::create::{EnvVar, Kind, Mount};
use objectiveai_sdk::cli::command::laboratories::list::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;
use objectiveai_sdk::podman::laboratory;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    // Only `Client` exists today; the match stays exhaustive so adding
    // `Server` later forces a decision here.
    match request.kind {
        Kind::Client => {}
    }

    let labs = laboratory::list(ctx.podman_runtime(), ctx.filesystem.state()).await;
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
                            .map(|(key, value)| EnvVar { key, value })
                            .collect(),
                        cwd: lab.cwd,
                    });
                }
            }
            Err(e) => yield Err(e.into()),
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
