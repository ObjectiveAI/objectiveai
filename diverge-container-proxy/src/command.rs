//! Commands, from the inside: `POST /command/agent`, one ask, the
//! items streamed back.
//!
//! The body is the command's bytes, opaque — the CLI's vocabulary,
//! which the proxy never reads. The answer is a streamed body of
//! RECORDS, `[len: u32 BE][message…]`, one per message the caller's
//! answer path carried — each a `command::response::Frame`, an item
//! or the error that ends them — and a record of zero length is the
//! END, written when the answer path closed cleanly. A body that
//! ends without it is the answer having died mid-stream.
//!
//! The status is decided by the FIRST event, before any body is
//! sent: a first message is `200` and the body begins with it; a
//! close with nothing before it is `502` — the wire's empty close,
//! the caller could not serve it, and a command that produced
//! nothing looks the same; a death is `502`; an ask that would not
//! encode is `500`. No retry, per the wire: a command has effects.

use std::convert::Infallible;
use std::sync::Arc;

use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use diverge_provider_sdk::container_proxy::command;
use diverge_provider_sdk::container_proxy::requests::request::Request;
use futures_util::StreamExt as _;
use futures_util::stream;

use crate::requests::{Event, Requests};

/// `POST /command/agent`.
pub async fn agent(State(requests): State<Arc<Requests>>, body: Bytes) -> Response {
    let Ok((_, mut receiver)) = requests
        .ask(Request::Command(command::request::Request(&body)))
        .await
    else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let first = loop {
        match receiver.recv().await {
            Some(Event::Message(bytes)) => break bytes,
            Some(Event::Complete | Event::Died) | None => {
                return StatusCode::BAD_GATEWAY.into_response();
            }
            // The postgres path's alone; never on a command's.
            Some(Event::Opened(_)) => {}
        }
    };
    let records = stream::unfold(
        (Some(first), receiver),
        |(first, mut receiver)| async move {
            if let Some(first) = first {
                return Some((record(&first), (None, receiver)));
            }
            loop {
                match receiver.recv().await {
                    Some(Event::Message(bytes)) => {
                        return Some((record(&bytes), (None, receiver)));
                    }
                    // The end record, then nothing: the finish that
                    // sent `Complete` dropped the sender, so the next
                    // poll finds the receiver closed.
                    Some(Event::Complete) => {
                        return Some((record(&[]), (None, receiver)));
                    }
                    Some(Event::Died) | None => return None,
                    Some(Event::Opened(_)) => {}
                }
            }
        },
    );
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
