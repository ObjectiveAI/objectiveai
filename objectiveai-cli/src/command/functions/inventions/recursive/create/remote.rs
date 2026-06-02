//! `functions inventions recursive create remote` — bare-naked
//! chunk-or-id streaming handler. Same shape as the alpha leaves,
//! except `state` is either an inline `ParamsState` or a remote-path
//! string parsed via [`crate::path_ref::PathRef`].

use std::pin::Pin;

use futures::Stream;
use futures::StreamExt;
use objectiveai_sdk::RemotePathCommitOptional;
use objectiveai_sdk::cli::command::functions::inventions::recursive::create::remote::{
    Request, RequestState, ResponseItem,
};
use objectiveai_sdk::functions::inventions::prompts::InlinePromptOrRemoteCommitOptional;
use objectiveai_sdk::functions::inventions::recursive::request::FunctionInventionRecursiveCreateParams;
use objectiveai_sdk::functions::inventions::state::ParamsStateOrRemoteCommitOptional;

use crate::context::Context;
use crate::error::Error;
use crate::path_ref::PathRef;
use crate::streaming::{InstanceItem, instance_subprocess_stream};

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let state = match request.state {
        RequestState::Inline(ps) => ParamsStateOrRemoteCommitOptional::Inline(ps),
        RequestState::Ref(s) => {
            let path = s.parse::<PathRef>().map_err(Error::PathParse)?.resolve()?;
            ParamsStateOrRemoteCommitOptional::Remote(path)
        }
    };

    let (agent, remote) = tokio::try_join!(
        super::resolve_agent(ctx, request.agent),
        super::read_inventions_remote(ctx),
    )?;

    let params = FunctionInventionRecursiveCreateParams {
        remote,
        overwrite: None,
        state,
        provider: None,
        agent,
        prompt: InlinePromptOrRemoteCommitOptional::Remote(
            RemotePathCommitOptional::Mock {
                name: "default".to_string(),
            },
        ),
        seed: request.seed,
        stream: Some(true),
        max_step_retries: None,
        continuation: request.continuation,
    };

    let stream = request
        .dangerous_advanced
        .as_ref()
        .and_then(|a| a.stream)
        .unwrap_or(false);

    let raw = instance_subprocess_stream(
        ctx,
        &["functions", "inventions", "recursive", "create", "remote"],
        &params,
        None,
        stream,
    );
    Ok(Box::pin(raw.map(map_item)))
}

fn map_item(item: Result<InstanceItem, Error>) -> Result<ResponseItem, Error> {
    match item? {
        InstanceItem::Id(id) => Ok(ResponseItem::Id(id)),
        InstanceItem::Chunk(value) => serde_json::from_value(value)
            .map(ResponseItem::Chunk)
            .map_err(|e| Error::InlineDeserialize(e.into())),
    }
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::functions::inventions::recursive::create::remote as sdk;
    use objectiveai_sdk::cli::command::functions::inventions::recursive::create::remote::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::functions::inventions::recursive::create::remote as sdk;
    use objectiveai_sdk::cli::command::functions::inventions::recursive::create::remote::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
