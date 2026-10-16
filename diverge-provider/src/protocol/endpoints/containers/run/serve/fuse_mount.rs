//! One more FUSE mount, made while the container runs: a mount scope
//! on the proxy, waited for, and the caller's channel answered once.

use std::sync::Arc;

use super::super::check;
use super::super::family::Runs;
use super::super::ledger::Refusal;
use super::super::relay;
use super::super::render;
use super::super::run::Run;
use super::super::setup::Mount;
use diverge_sdk::container_proxy::outside::endpoints::fuse::mount::client::execute as mount;
use diverge_sdk::shared::containers::fuse_mount::request::Request;
use diverge_sdk::shared::containers::fuse_mount::response::Frame;

/// Serve one, to the end. The rules, in order:
///
/// 1. A provider that holds out no `containers_fuse_mount` answers
///    `Unsupported` and does nothing else.
/// 2. A path that is the root, or has a component that is not a
///    name, is a request that did not decode: the finish alone.
/// 3. The mount is claimed in the run's [`Ledger`](super::super::ledger::Ledger)
///    before the proxy is asked: `Taken` for an id the container
///    holds a FUSE mount under, `Overlaps` for a path that is a held
///    mount's, lies inside one or holds one.
/// 4. One mount scope is opened on the proxy, and its answer waited
///    for. The proxy's `Ok` is the mount made: its path joins what
///    every tree opened from now leaves out, `Mounted` is sent and
///    the channel finished, and only then is the mount's relay
///    spawned — so that the answer precedes every ask of the id. The
///    proxy's refusal is the claim released and `Error`, the proxy's
///    words in it. The proxy's connection ending, or an answer that
///    could not be read, is the claim released and the finish alone.
///
/// The mount lives until the run ends; nothing unmounts it.
pub(crate) async fn fuse_mount<R: Runs>(run: Arc<Run>, channel: u32, request: Request) {
    if !run.fuse_mount {
        run.respond(channel, R::fuse_mount(&Frame::Unsupported)).await;
        run.finish(channel).await;
        return;
    }
    if request.container_path.is_empty() || !request.container_path.iter().all(|component| check::name(component)) {
        run.finish(channel).await;
        return;
    }
    let claimed = run.mounts.lock().await.claim(&request.id, &request.container_path);
    if let Err(refusal) = claimed {
        let frame = match refusal {
            Refusal::Overlaps => Frame::Overlaps,
            Refusal::Taken => Frame::Taken,
        };
        run.respond(channel, R::fuse_mount(&frame)).await;
        run.finish(channel).await;
        return;
    }
    match mount::execute(&run.proxy, request.container_path.clone(), request.kind).await {
        Ok((handle, asks)) => {
            run.ignore.lock().await.push(request.container_path);
            run.respond(channel, R::fuse_mount(&Frame::Mounted)).await;
            run.finish(channel).await;
            let made = Mount {
                id: request.id,
                handle,
                asks,
            };
            run.spawn(relay::fuse::<R>(Arc::clone(&run), made)).await;
        }
        Err(mount::ExecuteError::Refused(words)) => {
            run.mounts.lock().await.release(&request.id);
            run.respond(channel, R::fuse_mount(&Frame::Error(render::mount_failed(&request.id, words)))).await;
            run.finish(channel).await;
        }
        Err(_) => {
            run.mounts.lock().await.release(&request.id);
            run.finish(channel).await;
        }
    }
}
