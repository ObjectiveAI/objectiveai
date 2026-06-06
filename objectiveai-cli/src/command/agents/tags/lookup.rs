//! `agents tags lookup` — bare-naked handler.

use objectiveai_sdk::cli::command::agents::tags::lookup::{LookupState, Request, Response};

use crate::context::Context;
use crate::error::Error;
use crate::filesystem::db;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    match request {
        Request::AgentInstanceHierarchy {
            parent_agent_instance_hierarchy,
            agent_instance,
            ..
        } => {
            // Compose the full hierarchy from the leaf and the
            // explicit parent (or the cli's own ctx default).
            let parent = parent_agent_instance_hierarchy
                .unwrap_or_else(|| ctx.config.agent_instance_hierarchy.clone());
            let agent_instance_hierarchy = format!("{parent}/{agent_instance}");
            let tags = db::tags::tags_for_hierarchy_async(
                ctx.filesystem.clone(),
                agent_instance_hierarchy,
            )
            .await?;
            Ok(Response::AgentInstanceHierarchy { tags })
        }
        Request::Tag { tag, .. } => {
            let state = db::tags::lookup_async(ctx.filesystem.clone(), tag).await?;
            Ok(Response::Tag {
                state: db_to_sdk_state(state),
            })
        }
    }
}

fn db_to_sdk_state(state: db::tags::LookupState) -> LookupState {
    match state {
        db::tags::LookupState::Bound { agent_instance_hierarchy } => LookupState::Bound {
            agent_instance_hierarchy,
        },
        db::tags::LookupState::Pending {
            parent_agent_instance_hierarchy,
            agent_full_id,
        } => LookupState::Pending {
            parent_agent_instance_hierarchy,
            agent_full_id,
        },
        db::tags::LookupState::Absent => LookupState::Absent,
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
