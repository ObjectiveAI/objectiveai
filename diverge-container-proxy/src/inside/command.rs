//! Commands, from the inside: `POST /command`, one ask, the
//! items streamed back.
//!
//! The body is the command's bytes, opaque — the CLI's vocabulary,
//! which the proxy never reads. The answer is a streamed body of
//! RECORDS, `[len: u32 BE][message…]`, one per frame the caller's
//! answer carried — each a `command::response::Frame`, an item or
//! the error that ends them — and a record of zero length is the
//! END, written when the answer finished. A body that ends without
//! it is the answer having died mid-stream.
//!
//! The status is decided by the FIRST event, before any body is
//! sent: a first frame is `200` and the body begins with it; a finish
//! with nothing before it is `502` — the wire's empty finish, the
//! caller could not serve it, and a command that produced nothing
//! looks the same; a death is `502`; an ask that would not encode is
//! `500`. No retry, per the wire: a command has effects.

use std::convert::Infallible;
use std::sync::Arc;

use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use diverge_sdk::shared::containers::command;
use futures_util::StreamExt as _;
use futures_util::stream;

use crate::answer::{self, Answer};
use crate::ask::{self, Asked};
use crate::own::Own;
use crate::proxy::Proxy;

/// `POST /command`.
pub async fn agent(State(proxy): State<Arc<Proxy>>, body: Bytes) -> Response {
    let (_begun, mut channel) = match ask::open(&proxy, Own::Command(command::request::Request(&body))).await {
        Ok(opened) => opened,
        Err(Asked::Encode) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        Err(Asked::Empty | Asked::Died) => return StatusCode::BAD_GATEWAY.into_response(),
    };
    let first = match answer::next(&mut channel).await {
        Some(Answer::Frame(bytes)) => bytes,
        Some(Answer::Finish) | None => return StatusCode::BAD_GATEWAY.into_response(),
    };
    let records = stream::unfold((Some(first), Some(channel)), |(first, channel)| async move {
        if let Some(first) = first {
            return Some((record(&first), (None, channel)));
        }
        // After the end record there is nothing: the channel is
        // dropped, and the next poll finds no channel.
        let mut channel = channel?;
        match answer::next(&mut channel).await {
            Some(Answer::Frame(bytes)) => Some((record(&bytes), (None, Some(channel)))),
            Some(Answer::Finish) => Some((record(&[]), (None, None))),
            None => None,
        }
    });
    (
        StatusCode::OK,
        Body::from_stream(records.map(Ok::<Bytes, Infallible>)),
    )
        .into_response()
}

/// One record: the length, then the message.
fn record(message: &[u8]) -> Bytes {
    let len = u32::try_from(message.len()).unwrap_or(u32::MAX);
    let mut out = Vec::with_capacity(4 + message.len());
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(message);
    out.into()
}
