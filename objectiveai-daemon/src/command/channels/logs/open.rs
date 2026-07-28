//! `channels logs open` — reveal one entry's content by id, scoped to
//! the channel. Authenticate a channel secret; pure read.

use objectiveai_sdk::cli::command::channels::logs::open::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::db::channels;
use crate::error::Error;

pub async fn execute(
    global: &GlobalContext,
    _scoped: &ScopedContext,
    request: Request,
) -> Result<Response, Error> {
    let db = global.db_client().await?;
    let auth = channels::channel_auth(&db, &request.channel_id)
        .await?
        .ok_or_else(|| Error::Instance("no such channel".to_string()))?;
    // Either channel secret authorizes reads.
    auth.role_of(&request.secret).ok_or_else(|| {
        Error::Instance("secret does not authorize this channel".to_string())
    })?;
    match channels::read_content_by_id(&db, &request.channel_id, request.entry_id).await? {
        Some(entry) => {
            let identity = entry.identity;
            Ok(Response::Entry {
                id: entry.id,
                timestamp: crate::db::time::unix_to_rfc3339(entry.delivered_at),
                kind: super::list::to_kind(entry.direction),
                sender_agent_instance_hierarchy: identity.agent_instance_hierarchy,
                plugin_owner: identity.plugin_owner,
                plugin_name: identity.plugin_name,
                plugin_version: identity.plugin_version,
                content: entry.content,
            })
        }
        None => Ok(Response::NotFound),
    }
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::channels::logs::open as sdk;
    use objectiveai_sdk::cli::command::channels::logs::open::request_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(
        _global: &GlobalContext,
        _scoped: &ScopedContext,
        _request: Request,
    ) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::channels::logs::open as sdk;
    use objectiveai_sdk::cli::command::channels::logs::open::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(
        _global: &GlobalContext,
        _scoped: &ScopedContext,
        _request: Request,
    ) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
