//! Answering a listing, and keeping it answered, from a scope, a
//! manager and the word of changes.

use diverge_sdk::shared::Identity;
use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::broadcast;

use diverge_sdk::provider::endpoints::images::list::server::response;
use diverge_sdk::provider::endpoints::images::list::client::channel_request;
use crate::protocol::changes::Changes;
use crate::protocol::image_manager::ImageManager;
use diverge_sdk::provider::endpoints::info::server::response::Capabilities;
use diverge_sdk::shared::error::Error;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::encode::{Encode, Writer};
use diverge_sdk::wire::frame::client::ClientFrame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use crate::protocol::singles::Single;

/// Send the caller every image it owns, the word that the listing is
/// whole, and every change after, until the caller cancels.
///
/// In order:
///
/// 0. [`Unsupported`](response::Frame::Unsupported), then the finish,
///    on a provider whose capabilities keep no Store; nothing
///    is looked at.
/// 1. The changes subscribed to, before anything is read, so nothing
///    that happens meanwhile is missed; then the manager asked which
///    images the caller has. A manager that will not answer is the
///    [`Error`](response::Frame::Error), and the scope finishes.
/// 2. Every image listed sent as [`Added`](response::Frame::Added),
///    then exactly one [`Listed`](response::Frame::Listed) — at once,
///    with nothing before it, for a caller with no image.
/// 3. For the scope's life: a change to one of the caller's images,
///    heard by digest, is the manager asked again and the difference
///    told, keyed by digest — a digest not listed before as `Added`, a
///    digest listed and gone as [`Removed`](response::Frame::Removed)
///    with the image as it was, and a digest listed whose image
///    differs — its description edited — as
///    [`Changed`](response::Frame::Changed). A change to another
///    caller's image is nothing. A subscription that fell
///    behind is the manager asked again the same way. A manager that
///    will not answer then is the `Error`, last.
/// 4. The finish, when the caller's [`Cancel`](channel_request::Frame::Cancel)
///    arrives, the caller is gone or reads no more of the listing, or
///    the error was sent. The connection's claim on its one listing,
///    held with the scope, goes before it — see [`Single`].
///
/// A response that will not encode is the one failure with nowhere to
/// go, and is not sent; the listing goes on without it.
pub async fn handle<M>(
    scope: Single<ScopeHandle>,
    client_identity: &Identity,
    manager: &M,
    changes: Arc<Changes>,
    capabilities: &Capabilities,
) where
    M: ImageManager,
    M::Error: Into<Error>,
{
    if !capabilities.images_store {
        send(&scope, &response::Frame::Unsupported).await;
        scope.send_response_finish().await;
        return;
    }
    let mut nudges = changes.subscribe();
    let mut known: HashMap<String, response::Image> = HashMap::new();
    match manager.list(client_identity).await {
        Ok(images) => {
            for image in images {
                send(&scope, &response::Frame::Added(image.clone())).await;
                known.insert(image.digest.clone(), image);
            }
            send(&scope, &response::Frame::Listed).await;
        }
        Err(error) => {
            send(&scope, &response::Frame::Error(error.into())).await;
            scope.send_response_finish().await;
            return;
        }
    }
    loop {
        tokio::select! {
            nudge = nudges.recv() => {
                let again = match nudge {
                    Ok(changed) => changed.identity.as_ref() == client_identity,
                    Err(broadcast::error::RecvError::Lagged(_)) => true,
                    Err(broadcast::error::RecvError::Closed) => break,
                };
                if again {
                    match manager.list(client_identity).await {
                        Ok(images) => tell(&scope, &mut known, images).await,
                        Err(error) => {
                            send(&scope, &response::Frame::Error(error.into())).await;
                            break;
                        }
                    }
                }
            }
            request = scope.recv_channel_request() => {
                let Some(bytes) = request else {
                    break;
                };
                let Ok(ClientFrame::ChannelRequest { channel, payload, .. }) = ClientFrame::decode(&bytes) else {
                    continue;
                };
                match channel_request::Frame::decode(payload) {
                    Ok(channel_request::Frame::Cancel) => break,
                    Err(_) => scope.send_channel_response_finish(channel).await,
                }
            }
            // The caller reads no more of the listing: abandoned, and
            // over as at the cancel.
            () = scope.responses_gone() => break,
        }
    }
    scope.send_response_finish().await;
}

/// A image listing on a connection holding one open already: `AlreadyOpen`
/// and the finish, and nothing else — see
/// [the endpoints](diverge_sdk::provider::endpoints#one-open-at-a-time).
pub async fn refuse(scope: ScopeHandle) {
    send(&scope, &response::Frame::AlreadyOpen).await;
    scope.send_response_finish().await;
}

/// The listing as the manager has it now against the listing as
/// sent: every difference told, and the sent listing brought up to
/// date.
async fn tell(scope: &ScopeHandle, known: &mut HashMap<String, response::Image>, now: Vec<response::Image>) {
    let mut seen: HashMap<String, response::Image> = HashMap::with_capacity(now.len());
    for image in now {
        match known.remove(&image.digest) {
            None => send(scope, &response::Frame::Added(image.clone())).await,
            Some(was) if was != image => send(scope, &response::Frame::Changed(image.clone())).await,
            Some(_) => {}
        }
        seen.insert(image.digest.clone(), image);
    }
    for (_, was) in known.drain() {
        send(scope, &response::Frame::Removed(was)).await;
    }
    *known = seen;
}

/// One response on the scope, when it encoded.
async fn send(scope: &ScopeHandle, frame: &response::Frame) {
    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_response(&buffer).await;
    }
}
