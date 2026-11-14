//! A volume served over FUSE to the caller, through the daemon.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::volumes::fuse_serve::client::{channel_request, request};
use diverge_sdk::daemon::endpoints::volumes::fuse_serve::server::response::Frame;
use diverge_sdk::daemon::grant::volumes::Over;
use diverge_sdk::provider::endpoints::volumes::fuse_serve::client::execute::{ExecuteError, ExecuteHandle, execute};
use diverge_sdk::provider::endpoints::volumes::fuse_serve::client::request::Frame as Ask;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::frame::client::ClientFrame;
use diverge_sdk::wire::frame::server::ServerFrame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use tokio::task::JoinSet;

use super::{Located, locate};
use crate::containers::capable;
use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::reply;
use crate::store;
use crate::volumes;

/// Say whether the volume is served, and serve it: every ask the
/// caller opens on the scope carried to the provider's serve and its
/// answer carried back, until the caller cancels.
///
/// The rules:
///
/// 1. **Forbidden.** No standing, no `fuse_serve` grant over volumes at
///    all, or one that does not reach this volume, is `Forbidden`, and
///    the provider is asked nothing. `mount` is not this grant.
/// 2. **Not found.** A provider the daemon keeps no record of, or a
///    volume its provider does not list, is `NotFound`; a provider
///    that could not be asked is `Error`.
/// 3. **The provider's answer, relayed.** The provider's
///    `volumes::fuse_serve` of the volume is opened, as the daemon
///    itself, with the request's mode and layer size. A provider whose
///    capabilities serve no volume of that mode is `Unsupported`, and
///    is not asked. Its refusals — mounted, served already, held, in
///    another mode, unsupported — are answered as it gave them; its
///    error is the `Error`, in its words.
/// 4. **Serving, and in use.** Otherwise `Serving` is sent first, and
///    nothing more on the main stream until the finish. From before
///    that answer to after the scope's finish the daemon counts the
///    volume as served — see
///    [`fuse_serve_enter`](crate::daemon::Live::fuse_serve_enter) — so
///    a delete of it answers `InUse` and every request that takes it
///    at rest answers `Held`.
/// 5. **A true relay.** Each channel the caller opens is answered on
///    its own task. An ask is put on the provider's serve as the bytes
///    it came as, and the provider's answer sent back as the bytes it
///    came as, then the finish; an ask the provider's serve did not
///    answer is finished with nothing. A file tree is the provider's
///    file tree channel, every frame carried back as it comes, to its
///    finish. A channel request that will not decode is finished with
///    nothing.
/// 6. **The end.** The caller's cancel, the caller gone, the caller
///    reading no more of the scope, or the provider's serve ending
///    ends the loop; every answer still in flight is finished, save
///    that a caller reading no more is not waited on, and then the
///    scope. The provider's serve is cancelled after the finish, the
///    volume counted served no more, and its provider asked its
///    capacity again, the serve's room given back.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) {
    let served = match open(&scope, &frame, who, daemon).await {
        Ok(Ok(served)) => served,
        Ok(Err(answer)) => {
            reply::reply(&scope, &answer).await;
            scope.send_response_finish().await;
            return;
        }
        Err(error) => {
            reply::reply(&scope, &Frame::Error(reply::failure(&error))).await;
            scope.send_response_finish().await;
            return;
        }
    };
    // Rule 4: entered here and left once below, on the one path there
    // is from here to the end.
    daemon.live.fuse_serve_enter(&frame.volume).await;
    let capacity = daemon.live.capacity_cell(&frame.volume.provider);
    if let Some(cell) = &capacity {
        cell.nudge();
    }
    reply::reply(&scope, &Frame::Serving).await;
    // Rule 5.
    let scope = Arc::new(scope);
    let served = Arc::new(served);
    let mut tasks = JoinSet::new();
    loop {
        let bytes = tokio::select! {
            received = scope.recv_channel_request() => match received {
                Some(bytes) => bytes,
                None => break,
            },
            () = scope.responses_gone() => break,
            _ = served.wait() => break,
            Some(_) = tasks.join_next(), if !tasks.is_empty() => continue,
        };
        let Ok(ClientFrame::ChannelRequest { channel, payload, .. }) = ClientFrame::decode(&bytes) else {
            continue;
        };
        match channel_request::Frame::decode(payload) {
            Ok(channel_request::Frame::Cancel) => break,
            Ok(channel_request::Frame::Filetree) => {
                tasks.spawn(filetree(Arc::clone(&scope), Arc::clone(&served), channel));
            }
            Ok(channel_request::Frame::Ask(_)) => {
                tasks.spawn(ask(Arc::clone(&scope), Arc::clone(&served), channel, payload.to_vec()));
            }
            Err(_) => scope.send_channel_response_finish(channel).await,
        }
    }
    // Rule 6.
    let gone = tokio::select! {
        () = drain(&mut tasks) => false,
        () = scope.responses_gone() => true,
    };
    if gone {
        tasks.abort_all();
        drain(&mut tasks).await;
    }
    scope.send_response_finish().await;
    served.cancel().await;
    daemon.live.fuse_serve_leave(&frame.volume).await;
    if let Some(cell) = &capacity {
        cell.nudge();
    }
}

/// Rules 1 to 3: the provider's serve, held, or the answer that ends
/// the scope.
async fn open(scope: &ScopeHandle, frame: &request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<Result<ExecuteHandle, Frame>, store::Error> {
    let mut conn = daemon.store.acquire().await?;
    // Rule 1.
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        return Ok(Err(Frame::Forbidden));
    };
    if !judge::volumes::holds(&standing, Over::FuseServe) {
        return Ok(Err(Frame::Forbidden));
    }
    // Rule 2.
    let listed = match locate(&mut conn, daemon, &frame.volume).await? {
        Located::Volume(listed) => listed,
        Located::None => return Ok(Err(Frame::NotFound)),
        Located::Failed(error) => return Ok(Err(Frame::Error(reply::failure(&error)))),
    };
    drop(conn);
    if !judge::volumes::over(&standing, Over::FuseServe, &listed) {
        return Ok(Err(Frame::Forbidden));
    }
    scope.allowed().await;
    // Rule 3.
    let handle = match volumes::handle(daemon, &frame.volume.provider).await {
        Ok(handle) => handle,
        Err(fail) => return Ok(Err(Frame::Error(fail.error()))),
    };
    let lacked = daemon
        .live
        .capabilities(&frame.volume.provider)
        .await
        .and_then(|info| capable::fuse_serve(&info, frame.mode).err());
    if lacked.is_some() {
        return Ok(Err(Frame::Unsupported));
    }
    let ask = Ask {
        name: frame.volume.name.clone(),
        mode: frame.mode,
        overlay_disk: frame.overlay_disk.unwrap_or(0),
    };
    Ok(match execute(&handle, &ask).await {
        Ok(served) => Ok(served),
        Err(ExecuteError::Mounted) => Err(Frame::Mounted),
        Err(ExecuteError::FuseServed) => Err(Frame::FuseServed),
        Err(ExecuteError::Held) => Err(Frame::Held),
        Err(ExecuteError::VolumeMode(refused)) => Err(Frame::VolumeMode(refused)),
        Err(ExecuteError::Unsupported) => Err(Frame::Unsupported),
        Err(ExecuteError::Refused(error)) => Err(Frame::Error(error)),
        Err(error) => Err(Frame::Error(reply::failure(&error))),
    })
}

/// Every task of `tasks` waited for, to the last.
async fn drain(tasks: &mut JoinSet<()>) {
    while tasks.join_next().await.is_some() {}
}

/// One ask, relayed: the caller's bytes put on the provider's serve,
/// the provider's answer sent back as it came, then the finish. An ask
/// the serve did not answer is finished with nothing.
async fn ask(scope: Arc<ScopeHandle>, served: Arc<ExecuteHandle>, channel: u32, payload: Vec<u8>) {
    if let Ok(answer) = served.ask_raw(&payload).await {
        scope.send_channel_response(channel, &answer).await;
    }
    scope.send_channel_response_finish(channel).await;
}

/// The file tree, relayed: the provider's file tree channel opened,
/// and every frame it sends carried back as it comes, to its finish —
/// which comes with the serve's own end — or the caller's going.
async fn filetree(scope: Arc<ScopeHandle>, served: Arc<ExecuteHandle>, channel: u32) {
    if let Ok(mut tree) = served.filetree().await {
        loop {
            let bytes = tokio::select! {
                received = tree.response_receiver.recv() => match received {
                    Some(bytes) => bytes,
                    None => break,
                },
                () = scope.responses_gone() => break,
            };
            match ServerFrame::decode(&bytes) {
                Ok(ServerFrame::ChannelResponse { payload, .. }) => scope.send_channel_response(channel, payload).await,
                Ok(ServerFrame::ChannelResponseFinish { .. }) | Err(_) => break,
                Ok(_) => {}
            }
        }
    }
    scope.send_channel_response_finish(channel).await;
}
