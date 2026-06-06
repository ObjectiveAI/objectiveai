//! `agents read pending` — bare-naked streaming handler stub.

use std::pin::Pin;

use futures::Stream;
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use objectiveai_sdk::cli::command::agents::read::pending::{Request, ResponseItem, Target};

use crate::context::Context;
use crate::error::Error;
use crate::filesystem::db::tags;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let default_parent = ctx.config.agent_instance_hierarchy.clone();
    let fs = ctx.filesystem.clone();
    let stream = async_stream::stream! {
        let mut inflight = FuturesUnordered::new();
        for target in request.targets {
            let fs = fs.clone();
            let default_parent = default_parent.clone();
            inflight.push(async move {
                let (parent, spawned, leaf) = resolve_target(&fs, target, &default_parent).await?;
                let items = fs.read_new_from_queue(&parent, &spawned).await?;
                Ok::<_, Error>(ResponseItem { agent_id: leaf, items })
            });
        }
        while let Some(result) = inflight.next().await {
            yield result;
        }
    };
    Ok(Box::pin(stream))
}

/// Same as `agents read all`'s resolver — direct mode uses the
/// explicit `parent=` or ctx, tag mode looks the tag up in
/// `tags.sqlite` and errors on PENDING / ABSENT.
async fn resolve_target(
    fs: &crate::filesystem::Client,
    target: Target,
    default_parent: &str,
) -> Result<(String, String, String), Error> {
    match target {
        Target::Direct {
            parent_agent_instance_hierarchy,
            agent_instance,
        } => {
            let parent =
                parent_agent_instance_hierarchy.unwrap_or_else(|| default_parent.to_string());
            let spawned = format!("{parent}/{agent_instance}");
            Ok((parent, spawned, agent_instance))
        }
        Target::Tag { agent_tag } => match tags::lookup_async(fs.clone(), agent_tag.clone()).await? {
            tags::LookupState::Bound { agent_instance_hierarchy } => {
                let parent = tags::parent_of(&agent_instance_hierarchy).to_string();
                let leaf = tags::leaf_of(&agent_instance_hierarchy).to_string();
                Ok((parent, agent_instance_hierarchy, leaf))
            }
            tags::LookupState::Pending {
                parent_agent_instance_hierarchy,
                agent_full_id,
            } => Err(Error::TagPending {
                tag: agent_tag,
                parent_agent_instance_hierarchy,
                agent_full_id,
            }),
            tags::LookupState::Absent => Err(Error::TagNotFound(agent_tag)),
        },
    }
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::read::pending as sdk;
    use objectiveai_sdk::cli::command::agents::read::pending::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::read::pending as sdk;
    use objectiveai_sdk::cli::command::agents::read::pending::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::ResponseItem)))
    }
}
