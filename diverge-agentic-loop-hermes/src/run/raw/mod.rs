//! The run, raw: post it, subscribe, yield the gateway's events as
//! they are.
//!
//! # The wire (`reports/run-event-stream.md`)
//!
//! `POST /v1/runs` answers `202 {"run_id", "status": "started"}` and
//! emits nothing else; the run's events queue from that moment,
//! unbounded, whether anyone is listening. `GET
//! /v1/runs/{run_id}/events` is `text/event-stream`: data-only
//! frames, each one JSON discriminated by its own `event` key; a
//! `: keepalive` comment every 30 quiet seconds; a `: stream closed`
//! comment and then EOF when the run is over. The SSE parser drops
//! comments per the spec, so only data frames surface here, and EOF
//! is what ends the stream — a mid-run failure closes the socket
//! with no sentinel, and that is the stream ending too.
//!
//! # One subscriber, no reconnect
//!
//! Consumption is DESTRUCTIVE and the queue serves exactly one
//! reader: a second subscriber would split the events, and the
//! first to disconnect pops the queue for everybody. An unsubscribed
//! queue lives 300 seconds, then `404 run_not_found`. So this module
//! subscribes once, immediately after the `202`, and never retries a
//! broken stream — a reconnect would find the queue gone or steal
//! from nobody.
//!
//! Both HTTP exchanges carry `Authorization: Bearer <API_SERVER_KEY>`,
//! the key [`prepare`](crate::filesystem::prepare) minted. Failures
//! before the stream — auth (`401`), a bad request (`400`), the run
//! gone (`404`), the concurrency cap (`429`) — are plain JSON with a
//! status, and arrive as [`Error::Status`] before any stream exists.

mod body;
mod error;
mod request;
mod started;

pub use error::*;
pub use request::*;
pub use started::*;

use eventsource_stream::Eventsource as _;
use futures_util::{Stream, StreamExt as _};

use crate::filesystem;
use crate::filesystem::{API_SERVER_HOST, API_SERVER_PORT};
use crate::response::Event;

/// Start a run and stream its events.
///
/// A resume is two things, both from the database
/// [`prepare`](crate::filesystem::prepare) landed: the `session_id`
/// the request names (where the turn records), and that session's
/// transcript (what the model sees), which `/v1/runs` never loads
/// itself — so this reads it through
/// [`filesystem::history`] and sends it
/// as the body's `conversation_history`. A request naming no session
/// sends none.
///
/// The `202` and the subscription both happen before this returns,
/// so a run that cannot start is an `Err` and never a stream's
/// leading item. The stream then yields every data frame the
/// gateway writes, parsed as an [`Event`] — the twelve typed kinds,
/// or [`Unknown`](Event::Unknown) for a shape this crate has not
/// named — until EOF.
pub async fn run(
    api_server_key: &str,
    request: &Request,
) -> Result<impl Stream<Item = Result<Event, Error>>, Error> {
    let conversation_history = match &request.session_id {
        Some(session_id) => Some(filesystem::history(session_id).await?),
        None => None,
    };
    let body = body::Body {
        request,
        conversation_history,
    };

    let client = reqwest::Client::new();
    let bearer = format!("Bearer {api_server_key}");
    let base = format!("http://{API_SERVER_HOST}:{API_SERVER_PORT}/v1/runs");

    let response = client
        .post(&base)
        .header("authorization", &bearer)
        .json(&body)
        .send()
        .await?;
    let response = status(response).await?;
    let started: Started =
        serde_json::from_slice(&response.bytes().await?).map_err(Error::Started)?;

    let response = client
        .get(format!("{base}/{}/events", started.run_id))
        .header("authorization", &bearer)
        .send()
        .await?;
    let response = status(response).await?;
    let mut events = response.bytes_stream().eventsource();

    Ok(async_stream::try_stream! {
        while let Some(event) = events.next().await {
            let event = event.map_err(Error::Stream)?;
            let event: Event =
                serde_json::from_str(&event.data).map_err(Error::Frame)?;
            yield event;
        }
    })
}

/// A non-2xx answer is the failure it says, body kept verbatim — as
/// JSON when it is JSON, as text otherwise.
async fn status(response: reqwest::Response) -> Result<reqwest::Response, Error> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let text = response.text().await.unwrap_or_default();
    let body = serde_json::from_str(&text)
        .unwrap_or(serde_json::Value::String(text));
    Err(Error::Status {
        status: status.as_u16(),
        body,
    })
}
