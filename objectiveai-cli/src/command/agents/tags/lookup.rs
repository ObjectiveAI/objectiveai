//! `agents tags lookup` — bare-naked handler.

use objectiveai_sdk::cli::command::agents::tags::lookup::{LookupState, Request, Response};

use crate::context::Context;
use crate::db;
use crate::error::Error;

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
            let tags = db::tags::tags_for_hierarchy(
                ctx.db.get().await?,
                &agent_instance_hierarchy,
            )
            .await?;
            Ok(Response::AgentInstanceHierarchy { tags })
        }
        Request::Tag { tag, .. } => {
            let state = db::tags::lookup(ctx.db.get().await?, &tag).await?;
            Ok(match state {
                db::tags::LookupState::Bound { agent_instance_hierarchy } => Response::Tag {
                    state: LookupState::Bound { agent_instance_hierarchy },
                },
                db::tags::LookupState::Grouped {
                    tag_group_id,
                    agent_spec,
                    parent_agent_instance_hierarchy,
                } => Response::Tag {
                    state: LookupState::Grouped {
                        tag_group_id,
                        agent_spec,
                        parent_agent_instance_hierarchy,
                    },
                },
                db::tags::LookupState::Absent => Response::Absent,
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
