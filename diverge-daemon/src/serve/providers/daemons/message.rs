//! Sending one direct message through the daemon's providers.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

use futures_util::FutureExt as _;
use rand::seq::SliceRandom as _;

use diverge_sdk::daemon::endpoints::providers::daemons::message::client::{channel_request, request};
use diverge_sdk::daemon::endpoints::providers::daemons::message::server::response::Frame;
use diverge_sdk::daemon::grant::providers_daemons::Over;
use diverge_sdk::provider::endpoints::daemons::message::client::{execute as send, request as send_request};
use diverge_sdk::provider::endpoints::daemons::message::server::response as sent;
use diverge_sdk::shared::Identity;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::frame::client::ClientFrame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::{Daemon, Kind};
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store;

/// Send the message, and answer its fate: the answer, then the finish —
/// or, for a cancel in time, the finish alone.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) {
    let answer = match serve(&scope, frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Some(Frame::Error(reply::failure(&error))),
    };
    if let Some(answer) = answer {
        reply::reply(&scope, &answer).await;
    }
    scope.send_response_finish().await;
}

/// Why a provider is not tried again for this message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Excluded {
    /// The identity stopped listening there before it took the message.
    NotListening,
    /// The provider would not take the daemon's send: it relays no
    /// messages, a send is open on the connection already, or the
    /// connection to it is gone.
    Refused,
}

/// The rules:
///
/// 1. **Judged.** One `Forbidden` with no providers `message` grant.
/// 2. **Where.** Listings held ([`touch`](crate::listings::Listings::touch)),
///    the identity's routes read: none at all is `NotFound`; none
///    through a provider the grant reaches with `message` is
///    `Forbidden`; none of those through which the identity listens,
///    and not excluded for this message, is `NotListening` — or the
///    error, when every one of them was excluded because the provider
///    refused the daemon's send.
/// 3. **Which.** Among the candidates whose provider holds out
///    `messages`, the one with the most room in the daemon's window
///    whose lane is free — a tie broken at random — takes the message;
///    see [`Messaging`](crate::messaging::Messaging).
/// 4. **Waits for room.** With none free with room, the message waits
///    for a lane freed, the soonest full window's end, a word that
///    the listings or the providers changed, or the client's cancel — the
///    finish alone — and then goes back to rule 2. A message never
///    waits behind another to one provider while another has room.
/// 5. **Sent.** Counted as it goes, then the provider's answer read:
///    `Delivered` is the answer; `NotListening` excludes that provider
///    and goes back to rule 2; `RateLimited` holds that window full for
///    the seconds named, takes the count back, and goes back to rule 4;
///    a refusal of the daemon's send takes the count back, excludes the
///    provider, and goes back to rule 2; the finish with no answer, the
///    client having cancelled, is the finish alone; the connection
///    ending excludes the provider.
/// 6. **The cancel.** Before the message goes out, the client's cancel
///    is the finish alone, nothing counted. After it goes out, the
///    cancel is the provider send's own: the provider's finish alone is
///    the finish alone, and an answer it sends anyway is forwarded.
/// 7. **The lane** is let go before the scope's answer, however the
///    send ends.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<Option<Frame>, store::Error> {
    let standing = {
        let mut conn = daemon.store.acquire().await?;
        Standing::of(&mut conn, who).await?
    };
    // Rule 1.
    let Some(standing) = standing else {
        return Ok(Some(Frame::Forbidden));
    };
    if !judge::providers_daemons::holds(&standing, Over::Message) {
        return Ok(Some(Frame::Forbidden));
    }
    scope.allowed().await;
    let request::Frame { to, body, relay: _ } = frame;
    let messaging = &daemon.live.messaging;
    let mut excluded: Vec<(Identity, Excluded)> = Vec::new();
    let mut words = daemon.live.changes();
    let cancelled = cancel(scope).shared();
    loop {
        // Made before anything is looked at (rule 4).
        let freed = messaging.freed();
        let mut freed = std::pin::pin!(freed.notified());
        freed.as_mut().enable();
        // Rule 2.
        daemon.live.listings.touch(daemon).await;
        let routes = daemon.live.listings.routes(&to).await;
        if routes.is_empty() {
            return Ok(Some(Frame::NotFound));
        }
        let reached = judge::providers_daemons::reached(&standing, Over::Message, &daemon.providers.all());
        let reachable: Vec<_> = routes.into_iter().filter(|via| reached.contains(&via.provider)).collect();
        if reachable.is_empty() {
            return Ok(Some(Frame::Forbidden));
        }
        let listening: Vec<Identity> = reachable
            .iter()
            .filter(|via| via.listening && !excluded.iter().any(|(provider, _)| *provider == via.provider))
            .map(|via| via.provider.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        if listening.is_empty() {
            let refused = reachable.iter().filter(|via| via.listening).all(|via| excluded.contains(&(via.provider.clone(), Excluded::Refused)));
            if refused && reachable.iter().any(|via| via.listening) {
                return Ok(Some(Frame::Error(reply::failure(&"this daemon may not send through any provider the identity listens on"))));
            }
            return Ok(Some(Frame::NotListening));
        }
        // Rule 3: the candidates' room, the most first, ties at random.
        let mut candidates = Vec::new();
        let mut soonest: Option<Instant> = None;
        for provider in listening {
            let rate = daemon.live.capabilities(&provider).await.and_then(|info| info.capabilities.messages);
            let (Some(rate), Some(handle)) = (rate, daemon.live.provider(&provider).await) else {
                excluded.push((provider, Excluded::Refused));
                continue;
            };
            let (room, grows) = messaging.room(&provider, rate).await;
            if room == 0 {
                soonest = match (soonest, grows) {
                    (Some(a), Some(b)) => Some(a.min(b)),
                    (a, b) => a.or(b),
                };
                continue;
            }
            candidates.push((room, provider, rate, handle));
        }
        // Every listening provider excluded in this pass: rule 2 again,
        // which then answers.
        if candidates.is_empty() && soonest.is_none() {
            continue;
        }
        candidates.shuffle(&mut rand::rng());
        candidates.sort_by(|a, b| b.0.cmp(&a.0));
        let mut chosen = None;
        for (_, provider, rate, handle) in candidates {
            if let Some(lane) = messaging.claim(&provider).await {
                chosen = Some((provider, rate, handle, lane));
                break;
            }
        }
        // Rule 4.
        let Some((provider, rate, handle, lane)) = chosen else {
            let until = soonest.map(tokio::time::Instant::from_std);
            let mut window = std::pin::pin!(async {
                match until {
                    Some(until) => tokio::time::sleep_until(until).await,
                    None => std::future::pending().await,
                }
            });
            loop {
                tokio::select! {
                    () = &mut freed => break,
                    () = &mut window => break,
                    word = words.recv() => match word {
                        // A word of another kind changes nothing here.
                        Ok(kind) if kind != Kind::Listings && kind != Kind::Providers => {}
                        _ => break,
                    },
                    () = cancelled.clone() => return Ok(None),
                    () = scope.responses_gone() => return Ok(None),
                }
            }
            continue;
        };
        // Rule 5.
        messaging.count(&provider, rate).await;
        let opened = send::execute(&handle, &send_request::Frame { to: to.clone(), body: &body }).await;
        let Ok(sending) = opened else {
            drop(lane);
            messaging.uncount(&provider).await;
            excluded.push((provider, Excluded::Refused));
            continue;
        };
        let cancel_send = sending.cancel().clone();
        let mut waiting = std::pin::pin!(sending.wait());
        // Rule 6.
        let answered = tokio::select! {
            answered = &mut waiting => answered,
            () = cancelled.clone() => {
                cancel_send.cancel().await;
                waiting.await
            }
        };
        // Rule 7.
        drop(lane);
        match answered {
            Ok(Some(sent::Frame::Delivered)) => return Ok(Some(Frame::Delivered)),
            Ok(Some(sent::Frame::NotListening)) => excluded.push((provider, Excluded::NotListening)),
            Ok(Some(sent::Frame::RateLimited { retry_after })) => {
                messaging.uncount(&provider).await;
                messaging.full(&provider, retry_after).await;
            }
            Ok(Some(sent::Frame::Unsupported | sent::Frame::AlreadyOpen)) => {
                messaging.uncount(&provider).await;
                excluded.push((provider, Excluded::Refused));
            }
            // The finish alone: the cancel was in time, or the provider
            // did not serve the request.
            Ok(None) => {
                if cancelled.clone().now_or_never().is_some() {
                    return Ok(None);
                }
                messaging.uncount(&provider).await;
                excluded.push((provider, Excluded::Refused));
            }
            Err(_) => excluded.push((provider, Excluded::Refused)),
        }
    }
}

/// The client's one channel, read for its cancel and never answered;
/// the inbox closing is not a cancel.
async fn cancel(scope: &ScopeHandle) {
    loop {
        let Some(bytes) = scope.recv_channel_request().await else {
            return std::future::pending().await;
        };
        if let Ok(ClientFrame::ChannelRequest { payload, .. }) = ClientFrame::decode(&bytes)
            && let Ok(channel_request::Frame::Cancel) = channel_request::Frame::decode(payload)
        {
            return;
        }
    }
}
