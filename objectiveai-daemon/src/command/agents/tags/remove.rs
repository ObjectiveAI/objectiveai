//! `agents tags remove` — delete a tag registration by name,
//! whatever its shape, with its cleanup riding the same transaction
//! (laboratory attachments detached; an emptied `tag_groups` row
//! garbage-collected — see `db::tags::remove` for the race-safety
//! story). A missing tag is [`Error::TagNotFound`], matching the
//! apply/detach convention.

use objectiveai_sdk::cli::command::agents::tags::remove::{Removed, Request, Response};

use crate::context::Context;
use crate::db;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    // Resolve the db handle before taking the lock so an error there
    // can't skip the explicit release below.
    let pool = ctx.db_client().await?;
    // Same discipline as apply, same reason: a tag is load-bearing
    // identity, and deleting it out from under a live agent (whose
    // spawn holds the tag lock as part of its family) would rewrite
    // what that agent is called mid-flight. NON-BLOCKING: a held lock
    // means a live owner, and the remove is rejected.
    let state_dir = ctx.filesystem.state_dir();
    let (lock_dir, lock_key) =
        crate::command::agents::locks::agent_tag_lock(&state_dir, &request.tag);
    let Some(claim) =
        crate::command::agents::locks::try_acquire(ctx.agent_locks(), &lock_dir, &lock_key).await
    else {
        return Err(Error::TagRemoveAgentActive { tag: request.tag });
    };
    let result = db::tags::remove(pool, &request.tag).await;
    // Release the in-process guard before propagating the outcome.
    claim.release();
    match result? {
        db::tags::Removed::Absent => Err(Error::TagNotFound(request.tag)),
        db::tags::Removed::Bound {
            agent_instance_hierarchy,
            detached_laboratories,
        } => Ok(Response {
            name: request.tag,
            removed: Removed::Bound {
                agent_instance_hierarchy,
            },
            detached_laboratories,
        }),
        db::tags::Removed::Grouped {
            tag_group_deleted,
            detached_laboratories,
        } => Ok(Response {
            name: request.tag,
            removed: Removed::Grouped { tag_group_deleted },
            detached_laboratories,
        }),
    }
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::tags::remove as sdk;
    use objectiveai_sdk::cli::command::agents::tags::remove::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Request),
        ))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::tags::remove as sdk;
    use objectiveai_sdk::cli::command::agents::tags::remove::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Response),
        ))
    }
}
