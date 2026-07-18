//! `channels` tier — daemon-side dispatch for duplex channels.
//! `publish` (offer + block until accepted) and the `logs` sub-tier.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::AgentArguments;
use objectiveai_sdk::cli::command::channels::{Request, ResponseItem};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub mod logs;
pub mod publish;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

/// The daemon-authored agent identity for a channel offer/write — from
/// the caller's scope, plugin trio included (unspoofable; only
/// `plugins run` stamps it).
pub(crate) fn scope_identity(scoped: &ScopedContext) -> AgentArguments {
    AgentArguments {
        agent_instance_hierarchy: Some(scoped.agent_instance_hierarchy().to_string()),
        agent_id: scoped.agent_id().map(String::from),
        agent_full_id: scoped.agent_full_id().map(String::from),
        agent_remote: scoped.agent_remote().map(String::from),
        response_id: scoped.response_id().map(String::from),
        response_ids: scoped.response_ids().map(String::from),
        plugin_owner: scoped.plugin_owner().map(String::from),
        plugin_repository: scoped.plugin_repository().map(String::from),
        plugin_version: scoped.plugin_version().map(String::from),
    }
}

pub async fn execute(
    global: &GlobalContext,
    scoped: &ScopedContext,
    request: Request,
) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Publish(req) => {
            let value = publish::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::Publish(value)))
        }
        Request::PublishRequestSchema(req) => {
            let value = publish::request_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::PublishRequestSchema(value)))
        }
        Request::PublishResponseSchema(req) => {
            let value = publish::response_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::PublishResponseSchema(value)))
        }
        Request::Logs(req) => {
            let inner = logs::execute(global, scoped, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Logs)))
        }
    };
    Ok(stream)
}
