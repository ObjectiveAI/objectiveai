//! `channels logs subscribe` — return immediately if unread entries
//! exist; else block until a new matching entry arrives or the channel
//! closes, then return. Yields envelope items (advancing the role's
//! watermark), or the terminal `channel_closed`.
//!
//! Loop: (1) drain pending for this role and close; (2) if the channel
//! is closed with nothing pending, emit `channel_closed` and close;
//! (3) else wait on the LISTEN attached BEFORE step 1, then loop. The
//! pre-attached listener means a message/close landing during the
//! check is not lost.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::channels::logs::subscribe::{
    ChannelClosedTag, Request, ResponseItem,
};

use crate::context::{GlobalContext, ScopedContext};
use crate::db::channels::{self, ChannelState};
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(
    global: &GlobalContext,
    _scoped: &ScopedContext,
    request: Request,
) -> Result<ItemStream, Error> {
    let db = global.db_client().await?;
    let auth = channels::channel_auth(&db, &request.channel_id)
        .await?
        .ok_or_else(|| Error::Instance("no such channel".to_string()))?;
    let role = auth.role_of(&request.secret).ok_or_else(|| {
        Error::Instance("secret does not authorize this channel".to_string())
    })?;
    let channel_id = request.channel_id.clone();
    let after_id = request.after_id;
    let limit = request.limit;
    let stream = async_stream::stream! {
        // Attach BEFORE the first check so no wake is lost.
        let mut listener = match channels::channel_event_listener(&db).await {
            Ok(listener) => listener,
            Err(e) => {
                yield Err(e.into());
                return;
            }
        };
        loop {
            // 1. Drain any pending entries for this role, then close.
            match channels::any_pending(&db, &channel_id, role, after_id).await {
                Ok(true) => {
                    match channels::read_pending(&db, &channel_id, role, after_id, limit).await {
                        Ok(entries) => {
                            for envelope in entries {
                                yield Ok(ResponseItem::Item(super::list::to_entry(envelope)));
                            }
                            return;
                        }
                        Err(e) => {
                            yield Err(e.into());
                            return;
                        }
                    }
                }
                Ok(false) => {}
                Err(e) => {
                    yield Err(e.into());
                    return;
                }
            }
            // 2. Closed (or vanished) with nothing pending → terminal.
            match channels::channel_state(&db, &channel_id).await {
                Ok(Some(ChannelState::Open)) => {}
                Ok(Some(ChannelState::Closed)) | Ok(None) => {
                    yield Ok(ResponseItem::ChannelClosed(ChannelClosedTag::ChannelClosed));
                    return;
                }
                Err(e) => {
                    yield Err(e.into());
                    return;
                }
            }
            // 3. Wait for the next message or close, then loop.
            if let Err(e) = channels::recv_channel_event(&mut listener, &channel_id).await {
                yield Err(e.into());
                return;
            }
        }
    };
    Ok(Box::pin(stream))
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::channels::logs::subscribe as sdk;
    use objectiveai_sdk::cli::command::channels::logs::subscribe::request_schema::{
        Request, Response,
    };

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
    use objectiveai_sdk::cli::command::channels::logs::subscribe as sdk;
    use objectiveai_sdk::cli::command::channels::logs::subscribe::response_schema::{
        Request, Response,
    };

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
