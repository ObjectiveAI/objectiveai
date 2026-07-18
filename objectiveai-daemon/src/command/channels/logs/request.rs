//! `channels logs request` — the publisher→owner write. Authenticate
//! the publisher secret, require an OPEN channel, append a `request`
//! entry.

use objectiveai_sdk::cli::command::channels::logs::request::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::db::channels::{self, ChannelState, Direction, Role};
use crate::error::Error;

pub async fn execute(
    global: &GlobalContext,
    scoped: &ScopedContext,
    request: Request,
) -> Result<Response, Error> {
    let db = global.db_client().await?;
    let auth = channels::channel_auth(&db, &request.channel_id)
        .await?
        .ok_or_else(|| Error::Instance("no such channel".to_string()))?;
    let role = auth.role_of(&request.secret).ok_or_else(|| {
        Error::Instance("secret does not authorize this channel".to_string())
    })?;
    if role != Role::Publisher {
        return Err(Error::Instance(
            "channels logs request requires the publisher secret".to_string(),
        ));
    }
    if auth.state == ChannelState::Closed {
        return Ok(Response::ChannelClosed);
    }
    let identity = crate::command::channels::scope_identity(scoped);
    let (id, ts) = channels::insert_message(
        &db,
        &request.channel_id,
        Direction::Request,
        &identity,
        &request.content,
    )
    .await?;
    Ok(Response::Appended {
        id,
        timestamp: crate::db::time::unix_to_rfc3339(ts),
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::channels::logs::request as sdk;
    use objectiveai_sdk::cli::command::channels::logs::request::request_schema::{Request, Response};

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
    use objectiveai_sdk::cli::command::channels::logs::request as sdk;
    use objectiveai_sdk::cli::command::channels::logs::request::response_schema::{Request, Response};

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
