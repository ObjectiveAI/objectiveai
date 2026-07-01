//! `agents laboratories list` — stream the laboratory ids attached to an
//! agent target. Read-only: no locks. Resolves the target to its DB key
//! directly (tag → tag rows; instance → AIH rows; ref → error) and
//! streams `{ "id": "<id>" }` per attached laboratory.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::agents::laboratories::list::{Request, ResponseItem};
use objectiveai_sdk::cli::command::agents::selector::AgentSelector;

use crate::context::Context;
use crate::db::laboratory_attachments::Target;
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let target = match &request.selector {
        AgentSelector::Ref { .. } => return Err(Error::LaboratoryRefTarget),
        AgentSelector::Tag { agent_tag } => Target::Tag(agent_tag.clone()),
        AgentSelector::Instance {
            parent_agent_instance_hierarchy,
            agent_instance,
        } => {
            let parent = parent_agent_instance_hierarchy
                .as_deref()
                .unwrap_or(&ctx.config.agent_instance_hierarchy);
            Target::Aih(format!("{parent}/{agent_instance}"))
        }
    };
    let pool = ctx.db_client().await?.clone();
    let stream = async_stream::stream! {
        match crate::db::laboratory_attachments::list(&pool, &target).await {
            Ok(ids) => {
                for id in ids {
                    yield Ok(ResponseItem { id });
                }
            }
            Err(e) => yield Err(Error::from(e)),
        }
    };
    Ok(Box::pin(stream))
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::laboratories::list as sdk;
    use objectiveai_sdk::cli::command::agents::laboratories::list::request_schema::{
        Request, Response,
    };

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Request),
        ))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::laboratories::list as sdk;
    use objectiveai_sdk::cli::command::agents::laboratories::list::response_schema::{
        Request, Response,
    };

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::ResponseItem),
        ))
    }
}
