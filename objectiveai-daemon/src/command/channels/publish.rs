//! `channels publish` — create a pending offer on the `/channels`
//! hub, broadcast it to every connected stream, and BLOCK until the
//! first client accepts. Returns the new `channel_id` + the
//! publisher's secret (`S_pub`).
//!
//! The wait is uncapped here: the base `--timeout` is enforced by the
//! executor's `TimeoutStream`, which DROPS this future on expiry — the
//! [`AbandonGuard`] then withdraws the offer (notifying the connections
//! that saw it). A caller disconnect or early error is covered the same
//! way. Zero connected streams still waits; the offer replays to later
//! connections.

use objectiveai_sdk::cli::command::channels::publish::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

/// Maximum length of the offer message, in characters of the raw
/// (unescaped) string.
const MAX_MESSAGE_CHARS: usize = 512;

/// Withdraws the offer on drop unless disarmed — armed across the whole
/// accept wait.
struct AbandonGuard {
    hub: crate::http::channel_routes::ChannelHub,
    channel_id: String,
    armed: bool,
}

impl AbandonGuard {
    fn disarm(mut self) {
        self.armed = false;
    }
}

impl Drop for AbandonGuard {
    fn drop(&mut self) {
        if self.armed {
            self.hub.abandon_offer(&self.channel_id);
        }
    }
}

pub async fn execute(
    global: &GlobalContext,
    scoped: &ScopedContext,
    request: Request,
) -> Result<Response, Error> {
    let hubs = global.resident_hubs().ok_or_else(|| {
        Error::Instance("channels publish requires the resident daemon".to_string())
    })?;
    // A channel's PUBLISHER (its requester side) must be a plugin —
    // the trio is stored as the channel's origin and surfaces as the
    // required plugin identity on every `request` log entry.
    super::require_plugin(scoped, "publish")?;
    // The message caps at 512 CHARACTERS of the raw (unescaped)
    // string — enforced here so wire callers (who bypass the CLI's
    // clap parse) are held to it too.
    let message_len = request.message.chars().count();
    if message_len > MAX_MESSAGE_CHARS {
        return Err(Error::Channel(format!(
            "message exceeds {MAX_MESSAGE_CHARS} characters (got {message_len})"
        )));
    }
    let identity = super::scope_identity(scoped);
    let (channel_id, secret, rx) = hubs.channels.create_offer(
        request.key,
        request.details,
        request.message,
        scoped.plugin_owner().map(String::from),
        scoped.plugin_name().map(String::from),
        scoped.plugin_version().map(String::from),
        identity,
    );
    let guard = AbandonGuard {
        hub: hubs.channels.clone(),
        channel_id: channel_id.clone(),
        armed: true,
    };
    // Unblocks when a client accepts (the daemon persisted the channel
    // and pushed the owner secret over that connection's SSE). A dropped
    // sender (accept's DB insert failed) surfaces as an error.
    rx.await.map_err(|_| {
        Error::Instance("channel offer ended without acceptance".to_string())
    })?;
    guard.disarm();
    Ok(Response { channel_id, secret })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::channels::publish as sdk;
    use objectiveai_sdk::cli::command::channels::publish::request_schema::{Request, Response};

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
    use objectiveai_sdk::cli::command::channels::publish as sdk;
    use objectiveai_sdk::cli::command::channels::publish::response_schema::{Request, Response};

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
