//! `agents tags lookup` — bare-naked handler.

use objectiveai_sdk::cli::command::agents::tags::lookup::{LookupState, Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::db;
use crate::error::Error;

pub async fn execute(global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<Response, Error> {
    match request {
        Request::AgentInstanceHierarchy {
            parent_agent_instance_hierarchy,
            agent_instance,
            ..
        } => {
            // Compose the full hierarchy from the leaf and the
            // explicit parent (or the cli's own ctx default).
            let parent = parent_agent_instance_hierarchy
                .unwrap_or_else(|| scoped.agent_instance_hierarchy().to_string());
            let agent_instance_hierarchy = format!("{parent}/{agent_instance}");
            let tags = db::tags::tags_for_hierarchy(
                &global.db_client().await?,
                &agent_instance_hierarchy,
            )
            .await?;
            Ok(Response::AgentInstanceHierarchy { tags })
        }
        Request::Tag { tag, .. } => {
            let state = db::tags::lookup(&global.db_client().await?, &tag).await?;
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

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Request),
        ))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::tags::lookup as sdk;
    use objectiveai_sdk::cli::command::agents::tags::lookup::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Response),
        ))
    }
}
