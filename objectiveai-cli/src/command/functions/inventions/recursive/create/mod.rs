//! `functions inventions recursive create` sub-tier. AlphaScalar,
//! AlphaVector, and Remote are chunk-or-id streaming leaves.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::Remote;
use objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
use objectiveai_sdk::cli::command::agents::instances::spawn::AgentSpec;
use objectiveai_sdk::cli::command::functions::inventions::recursive::create::{
    Request, ResponseItem,
};

use crate::context::Context;
use crate::error::Error;

pub mod alpha_scalar;
pub mod alpha_vector;
pub mod remote;

pub(super) async fn resolve_agent(
    ctx: &Context,
    spec: AgentSpec,
) -> Result<InlineAgentBaseWithFallbacksOrRemoteCommitOptional, Error> {
    match spec {
        AgentSpec::Resolved(r) => Ok(r),
        AgentSpec::Favorite(name) => {
            let mut config = ctx.filesystem.read_config().await?;
            let fav = config
                .agents()
                .get_favorites()
                .iter()
                .find(|f| f.get_name() == name)
                .ok_or_else(|| Error::FavoriteNotFound(name.clone()))?;
            Ok(InlineAgentBaseWithFallbacksOrRemoteCommitOptional::Remote(
                fav.path.clone(),
            ))
        }
    }
}

/// Read `functions.inventions.remote` from on-disk config. Hardcoded
/// by the legacy dispatcher, not exposed via the SDK leaf's `Request`.
pub(super) async fn read_inventions_remote(ctx: &Context) -> Result<Remote, Error> {
    let mut config = ctx.filesystem.read_config().await?;
    Ok(config.functions().inventions().get_remote())
}

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::AlphaScalar(req) => {
            let inner = alpha_scalar::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::AlphaScalar)))
        }
        Request::AlphaScalarRequestSchema(req) => {
            let value = alpha_scalar::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::AlphaScalarRequestSchema(value)))
        }
        Request::AlphaScalarResponseSchema(req) => {
            let value = alpha_scalar::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::AlphaScalarResponseSchema(value)))
        }
        Request::AlphaVector(req) => {
            let inner = alpha_vector::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::AlphaVector)))
        }
        Request::AlphaVectorRequestSchema(req) => {
            let value = alpha_vector::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::AlphaVectorRequestSchema(value)))
        }
        Request::AlphaVectorResponseSchema(req) => {
            let value = alpha_vector::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::AlphaVectorResponseSchema(value)))
        }
        Request::Remote(req) => {
            let inner = remote::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Remote)))
        }
        Request::RemoteRequestSchema(req) => {
            let value = remote::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::RemoteRequestSchema(value)))
        }
        Request::RemoteResponseSchema(req) => {
            let value = remote::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::RemoteResponseSchema(value)))
        }
    };
    Ok(stream)
}
