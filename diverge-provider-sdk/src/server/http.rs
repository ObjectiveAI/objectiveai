//! One HTTP request, over the byte pipe into a container.
//!
//! Private, and meant to stay private. It is plumbing between two
//! things this crate owns — a [`Container`] and a handler — rather than
//! anything a provider implements or calls, and making it public would
//! put `hyper` types in this crate's signatures and make the version
//! semver-visible.
//!
//! # Why hyper rather than writing it
//!
//! Because the part that matters is not the request. Writing `POST
//! /path HTTP/1.1` and some headers is a `format!`; reading the answer
//! is a head parser that has to survive arriving in pieces, and then a
//! chunked-transfer decoder, which is where a streaming response
//! actually lives. That decoder is small, fiddly, and exactly the kind
//! of code that is wrong at the boundaries under load — and this repo
//! runs no tests.
//!
//! hyper is already compiled here: axum's `ws` brings it. Two features
//! and it does the whole answer correctly.
//!
//! # It does not serve
//!
//! Which is the line [`server`](super) draws and this does not cross. A
//! provider owns its endpoint, its URL space and its router; this dials
//! a pipe that a provider handed over, and speaks into it.

use std::io;
use std::sync::{Arc, Mutex};

use bytes::Bytes;
use futures_util::{SinkExt as _, StreamExt as _};
use http_body_util::Full;
use hyper::body::Incoming;
use hyper_util::rt::TokioIo;
use tokio_util::io::{CopyToBytes, SinkWriter, StreamReader};

use super::container::Container;
use crate::shared::error::Error;

/// Open a connection to a port inside a container and send one request
/// on it.
///
/// The response comes back with its body unread, so a caller streams it
/// rather than waiting for it — which is the whole point here, the one
/// caller being a loop that relays chunks as they arrive.
///
/// # The connection outlives this call
///
/// hyper splits a connection into a sender and a task that drives it,
/// and the second has to keep being polled for the first to make
/// progress. That task is spawned, and it ends when the body is
/// finished or dropped.
///
/// So a caller that drops the response drops the connection with it,
/// and a caller that reads to the end lets both go. Neither leaks; the
/// container is stopped separately and by name, because a pipe closing
/// is not a container exiting.
///
/// # What the errors are
///
/// [`Err`] means no answer at all — the port refused, the pipe broke
/// before a head arrived, the container answered something that was not
/// HTTP. A response with a status in it is [`Ok`], including a `500`:
/// what a status MEANS is the caller's to judge, and this layer would
/// be guessing.
pub async fn request<C>(
    container: &C,
    port: u16,
    request: hyper::Request<Full<Bytes>>,
) -> Result<hyper::Response<Incoming>, Error>
where
    C: Container,
    C::Error: Into<Error>,
{
    let (reader, writer) =
        container.connect(port).await.map_err(Into::into)?;

    // The pipe's own failures cannot travel as themselves: the adapters
    // below need `io::Error`, and a container's error type promises
    // nothing that could be formatted into one. So the real error is
    // put aside as it goes past, and read back if hyper then complains.
    let stashed = Arc::new(Mutex::new(None));

    let reading = Arc::clone(&stashed);
    let reader = StreamReader::new(reader.map(move |result| {
        result.map_err(|error| {
            stash(&reading, error.into());
            io::Error::other("the container's pipe failed")
        })
    }));
    let writing = Arc::clone(&stashed);
    let writer = SinkWriter::new(CopyToBytes::new(writer.sink_map_err(
        move |error| {
            stash(&writing, error.into());
            io::Error::other("the container's pipe failed")
        },
    )));

    let (mut sender, connection) =
        hyper::client::conn::http1::handshake(TokioIo::new(
            tokio::io::join(reader, writer),
        ))
        .await
        .map_err(|error| taken(&stashed, error))?;

    // Dropped by the sender going away, which happens when the response
    // body is finished or dropped.
    tokio::spawn(connection);

    sender
        .send_request(request)
        .await
        .map_err(|error| taken(&stashed, error))
}

/// Keep the first pipe failure, and only the first.
///
/// The first is the one that explains the rest: a broken pipe reports
/// itself again on every subsequent poll, and the later reports say
/// nothing the first did not.
fn stash(slot: &Mutex<Option<Error>>, error: Error) {
    if let Ok(mut slot) = slot.lock() {
        slot.get_or_insert(error);
    }
}

/// The pipe's failure if there was one, and hyper's if there was not.
///
/// hyper reports a broken connection in its own words, which are true
/// and unhelpful — the container's error is the one that says what
/// actually went wrong, and it is preferred whenever it exists.
fn taken(slot: &Mutex<Option<Error>>, fallback: hyper::Error) -> Error {
    slot.lock()
        .ok()
        .and_then(|mut slot| slot.take())
        .unwrap_or_else(|| {
            Error(serde_json::Value::String(fallback.to_string()))
        })
}
