//! A frontend request to a tool's program, served for as long as the
//! scope is open.

use std::convert::Infallible;
use std::sync::Arc;

use bytes::Bytes;
use diverge_sdk::daemon::endpoints::images::info::server::response::Frame as Info;
use diverge_sdk::daemon::endpoints::standalone_tools::frontend::client::channel_request;
use diverge_sdk::shared::containers::frontend;
use diverge_sdk::shared::containers::request::Image;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::encode::Encode;
use diverge_sdk::wire::frame::client::ClientFrame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use futures_util::StreamExt as _;
use futures_util::stream::FuturesUnordered;
use tokio::sync::mpsc;

use crate::containers::{Opened, images};
use crate::daemon::Daemon;
use crate::serve::reply;
use crate::store;

/// Whether the image's `/DIVERGE.md` declares a frontend: `Ok(true)`
/// where it says so, `Ok(false)` where it does not or the image ships
/// none, and `Err` with the reason where the document could not be
/// read.
pub async fn declares(daemon: &Arc<Daemon>, image: &Image) -> Result<Result<bool, String>, store::Error> {
    Ok(match images::info(daemon, image).await? {
        Info::Info(document) => Ok(document.frontend),
        Info::Missing => Ok(false),
        Info::NoSource => Err("the image's document could not be read: no source holds the image".to_string()),
        Info::Forbidden => Err("the image's document could not be read".to_string()),
        Info::Error(error) => Err(format!("the image's document could not be read: {}", error.0)),
    })
}

/// Send `head` to the program's frontend in the container, the body the
/// client's channels give, and send every frame of the answer, as the
/// family's answer `R`.
///
/// The rules:
///
/// 1. Each frame of the answer goes on as it comes, paced by the
///    client's reading.
/// 2. `Body` and `End` are served as they arrive, beside the answer:
///    each piece is handed to the request's body, one ahead, and its
///    channel finished once taken; `End` ends the body. A channel that
///    does not decode is finished and does nothing.
/// 3. The scope ends at the answer's end, at the client's `Cancel`, or
///    at the client gone; a stream that breaks is the error, after what
///    was sent. Its end drops the request.
/// 4. A body the provider never asked for is taken back at the end.
/// 5. The [`Opened`] is held for the whole of it: the request is a use
///    of the container, which holds it up.
///
/// Every family's channels are the same bytes, read here as the
/// standalone tools'.
pub async fn frontend<R>(scope: &ScopeHandle, opened: &Opened, head: frontend::request::Head) -> Result<(), String>
where
    R: From<frontend::output::Owned> + Encode,
{
    let (pieces, taken) = mpsc::channel::<Bytes>(1);
    let body = futures_util::stream::unfold(taken, |mut taken| async move { taken.recv().await.map(|piece| (Ok::<Bytes, Infallible>(piece), taken)) });
    let (mut output, write_id) = opened.frontend(head, body).await?;
    let ended = relay::<R>(scope, &mut output, pieces).await;
    // Rule 4.
    opened.forget(write_id).await;
    ended
}

/// Rules 1 to 3: the answer out, the body in.
async fn relay<R>(scope: &ScopeHandle, output: &mut crate::containers::FrontendFrames, pieces: mpsc::Sender<Bytes>) -> Result<(), String>
where
    R: From<frontend::output::Owned> + Encode,
{
    let mut pieces = Some(pieces);
    let mut serving = FuturesUnordered::new();
    loop {
        tokio::select! {
            frame = output.next() => match frame {
                // Rule 1.
                Some(Ok(frame)) => reply::reply(scope, &R::from(frame)).await,
                // Rule 3.
                Some(Err(error)) => return Err(error),
                None => return Ok(()),
            },
            request = scope.recv_channel_request() => {
                // Rule 3: the client gone.
                let Some(bytes) = request else {
                    return Ok(());
                };
                let Ok(ClientFrame::ChannelRequest { channel, payload, .. }) = ClientFrame::decode(&bytes) else {
                    continue;
                };
                match channel_request::Frame::decode(payload) {
                    // Rule 3.
                    Ok(channel_request::Frame::Cancel) => return Ok(()),
                    // Rule 2.
                    Ok(channel_request::Frame::Body(bytes)) => match &pieces {
                        Some(sender) => {
                            let sender = sender.clone();
                            let piece = Bytes::copy_from_slice(bytes);
                            serving.push(async move {
                                let _ = sender.send(piece).await;
                                channel
                            });
                        }
                        None => scope.send_channel_response_finish(channel).await,
                    },
                    Ok(channel_request::Frame::End) => {
                        pieces = None;
                        scope.send_channel_response_finish(channel).await;
                    }
                    Err(_) => scope.send_channel_response_finish(channel).await,
                }
            }
            Some(channel) = serving.next(), if !serving.is_empty() => scope.send_channel_response_finish(channel).await,
        }
    }
}
