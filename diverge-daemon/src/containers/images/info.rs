//! An image's `/DIVERGE.md`: the daemon's record of it, and the read
//! that fills the record.

use std::sync::Arc;

use diverge_sdk::provider::endpoints::containers::client::UnaryError;
use diverge_sdk::provider::endpoints::containers::client::answered::Info;
use diverge_sdk::shared::containers::info::{Document, response};
use diverge_sdk::shared::containers::request::Image;
use diverge_sdk::shared::error::Error;

use super::{Pulls, Reach, locate};
use crate::daemon::Daemon;
use crate::serve::reply;
use crate::store;

/// What a read of an image's document came to.
#[derive(Debug, Clone, PartialEq)]
pub enum Read {
    /// The document, parsed.
    Info(Document),
    /// The image ships none.
    Missing,
    /// No source in reach holds the image, so nothing was read.
    NoSource,
    /// The source that holds the image could not read the document,
    /// or the document is none.
    Error(Error),
}

/// The image's `/DIVERGE.md`, for the daemon's own use: what a create,
/// a deployment or a frontend asks before it goes on. The rules:
///
/// 1. **The record first.** A document the daemon has read before is
///    on its record by the image's digest — a digest is the image, so
///    an answer found once is the answer for good — and answers at
///    once, with nobody asked.
/// 2. **Else the one source.** Otherwise the daemon asks, as
///    [`locate`] asks, every provider and every remote daemon in reach
///    whether it holds the image — the daemon asking for itself, so
///    its reach is whole — and the first that does is asked for the
///    document on the `info` channel of that `images::read` scope, and
///    the scope cancelled. No source is [`Read::NoSource`]. No
///    container is asked, running or not: a document is read of the
///    image.
/// 3. **Only a document is recorded.** An [`Read::Info`] is saved to
///    the record; that an image ships none, that no source holds it,
///    and a failure are each asked again the next time.
pub async fn info(daemon: &Arc<Daemon>, image: &Image) -> Result<Read, store::Error> {
    // Rule 1.
    if let Some(found) = recorded(daemon, &image.digest).await? {
        return Ok(Read::Info(found));
    }
    // Rule 2.
    let Some(source) = locate(daemon, image, &Reach::Full { except: None }, &[]).await else {
        return Ok(Read::NoSource);
    };
    let read = asked(source.scope.info().await);
    let _ = source.scope.cancel().await;
    // Rule 3.
    if let Read::Info(found) = &read {
        record(daemon, &image.digest, found).await?;
    }
    Ok(read)
}

/// The image's `/DIVERGE.md`, for a client's `info` channel on an
/// `images::read` scope the daemon holds open through `pulls`: the
/// answer as the channel carries it. The rules of [`info`], save that
/// the source is the scope's own, already held: the record first, else
/// that one source's `info` channel, and a document found is recorded.
/// With no source held any more, the answer is an error.
pub async fn through(daemon: &Arc<Daemon>, pulls: &Pulls, digest: &str) -> response::Frame {
    match recorded(daemon, digest).await {
        Ok(Some(found)) => return response::Frame::Info(found),
        Ok(None) => {}
        Err(error) => return response::Frame::Error(reply::failure(&error)),
    }
    let Some(source) = pulls.held().await else {
        return response::Frame::Error(reply::failure(&"no source holds the image any more"));
    };
    match asked(source.scope.info().await) {
        Read::Info(found) => {
            if let Err(error) = record(daemon, digest, &found).await {
                return response::Frame::Error(reply::failure(&error));
            }
            response::Frame::Info(found)
        }
        Read::Missing => response::Frame::Missing,
        Read::NoSource => response::Frame::Error(reply::failure(&"no source holds the image any more")),
        Read::Error(error) => response::Frame::Error(error),
    }
}

/// The image's document, or why there is none to go by, in a
/// sentence: what a create and a deployment refuse with.
pub async fn document(daemon: &Arc<Daemon>, image: &Image) -> Result<Result<Document, String>, store::Error> {
    Ok(match info(daemon, image).await? {
        Read::Info(document) => Ok(document),
        Read::Missing => Err("the image ships no /DIVERGE.md".to_string()),
        Read::NoSource => Err("the image's document could not be read: no source holds the image".to_string()),
        Read::Error(error) => Err(format!("the image's document could not be read: {}", error.0)),
    })
}

/// The record's document for `digest`, if it has one.
async fn recorded(daemon: &Daemon, digest: &str) -> Result<Option<Document>, store::Error> {
    let mut conn = daemon.store.acquire().await?;
    store::image_info::by_digest(&mut conn, digest).await
}

/// `document` put on the record for `digest`.
async fn record(daemon: &Daemon, digest: &str, document: &Document) -> Result<(), store::Error> {
    let mut conn = daemon.store.acquire().await?;
    store::image_info::save(&mut conn, digest, document).await
}

/// A source's answer on its `info` channel, as a [`Read`]: the
/// source's refusal in its own words, and a channel that broke in the
/// daemon's.
fn asked(read: Result<Option<Document>, UnaryError<Info>>) -> Read {
    match read {
        Ok(Some(document)) => Read::Info(document),
        Ok(None) => Read::Missing,
        Err(UnaryError::Refused(error)) => Read::Error(error),
        Err(error) => Read::Error(reply::failure(&error)),
    }
}
