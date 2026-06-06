//! `agents tags lookup` — bare-naked handler.

use objectiveai_sdk::cli::command::agents::tags::lookup::{Request, Response};

use crate::context::Context;
use crate::error::Error;
use crate::filesystem::db;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    match request {
        Request::AgentInstanceHierarchy {
            agent_instance_hierarchy,
            ..
        } => {
            let tag = db::tags::tag_for_hierarchy_async(
                ctx.filesystem.clone(),
                agent_instance_hierarchy,
            )
            .await?;
            Ok(Response::AgentInstanceHierarchy { tag })
        }
        Request::Tag { tag, .. } => {
            let hierarchy =
                db::tags::hierarchy_for_tag_async(ctx.filesystem.clone(), tag).await?;
            Ok(Response::Tag {
                agent_instance_hierarchy: hierarchy,
            })
        }
    }
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::tags::lookup as sdk;
    use objectiveai_sdk::cli::command::agents::tags::lookup::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Request),
        ))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::tags::lookup as sdk;
    use objectiveai_sdk::cli::command::agents::tags::lookup::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Response),
        ))
    }
}
