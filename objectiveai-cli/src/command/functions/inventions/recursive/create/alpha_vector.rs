//! `functions inventions recursive create alpha_vector` — bare-naked
//! chunk-or-id streaming handler. Identical shape to `alpha_scalar`
//! with `ParamsState::AlphaVector` instead of `AlphaScalar`.

use std::pin::Pin;

use futures::Stream;
use futures::StreamExt;
use objectiveai_sdk::RemotePathCommitOptional;
use objectiveai_sdk::cli::command::functions::inventions::recursive::create::alpha_vector::{
    Request, RequestParams, ResponseItem,
};
use objectiveai_sdk::functions::inventions::prompts::InlinePromptOrRemoteCommitOptional;
use objectiveai_sdk::functions::inventions::recursive::request::FunctionInventionRecursiveCreateParams;
use objectiveai_sdk::functions::inventions::state::{
    AlphaVectorState, Params, ParamsState, ParamsStateOrRemoteCommitOptional,
};

use crate::context::Context;
use crate::error::Error;
use crate::streaming::{InstanceItem, instance_subprocess_stream};

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let (agent, remote) = tokio::try_join!(
        super::resolve_agent(ctx, request.agent),
        super::read_inventions_remote(ctx),
    )?;

    let state = ParamsStateOrRemoteCommitOptional::Inline(ParamsState::AlphaVector(
        AlphaVectorState {
            params: params_from_request(request.params),
            input_schema: None,
        },
    ));

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
        crate::instance::request::InstanceEndpoint::FunctionsInventionsRecursiveCreate(params),
        None,
        stream,
    );
    Ok(Box::pin(raw.map(map_item)))
}

fn params_from_request(p: RequestParams) -> Params {
    Params {
        depth: p.depth,
        min_branch_width: p.min_branch_width,
        max_branch_width: p.max_branch_width,
        min_leaf_width: p.min_leaf_width,
        max_leaf_width: p.max_leaf_width,
        name: p.name,
        spec: p.spec,
    }
}

fn map_item(item: Result<InstanceItem, Error>) -> Result<ResponseItem, Error> {
    match item? {
        InstanceItem::Id(id) => Ok(ResponseItem::Id(id)),
        InstanceItem::Chunk(value) => serde_json::from_value(value)
            .map(ResponseItem::Chunk)
            .map_err(|e| Error::InlineJson(e)),
    }
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::functions::inventions::recursive::create::alpha_vector as sdk;
    use objectiveai_sdk::cli::command::functions::inventions::recursive::create::alpha_vector::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::functions::inventions::recursive::create::alpha_vector as sdk;
    use objectiveai_sdk::cli::command::functions::inventions::recursive::create::alpha_vector::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
