//! `channels close` — close a channel (terminal: no further requests
//! or replies; the log lingers, readable and listable). EITHER
//! channel secret authorizes it. Idempotent — closing a closed
//! channel succeeds. The state change fires the `channel_closed`
//! NOTIFY, so any blocked `channels logs subscribe` unblocks with
//! `channel_closed`.

use objectiveai_sdk::cli::command::channels::close::{Request, Response};

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
    // Either channel secret authorizes the close — the requester and
    // the replier can each end the conversation.
    auth.role_of(&request.secret).ok_or_else(|| {
        Error::Instance("secret does not authorize this channel".to_string())
    })?;
    channels::close_channel(&db, &request.channel_id).await?;
    Ok(Response {
        channel_id: request.channel_id,
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::channels::close as sdk;
    use objectiveai_sdk::cli::command::channels::close::request_schema::{Request, Response};

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
    use objectiveai_sdk::cli::command::channels::close as sdk;
    use objectiveai_sdk::cli::command::channels::close::response_schema::{Request, Response};

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
