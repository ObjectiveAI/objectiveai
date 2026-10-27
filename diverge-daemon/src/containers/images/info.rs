//! An image's DIVERGE.md: from a container of it running, or from a
//! provider that can obtain it.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::daemon::endpoints::images::info::server::response::Frame;
use diverge_sdk::provider::endpoints::containers::client::UnaryError;
use diverge_sdk::provider::endpoints::containers::info::client::{execute, request};
use diverge_sdk::shared::containers::info::Document;
use diverge_sdk::shared::containers::request::Image;
use diverge_sdk::shared::mcp::attestation;
use rand::Rng as _;
use rand::seq::IndexedRandom as _;

use super::locate;
use crate::containers::capable;
use crate::containers::run::ToolHandle;
use crate::daemon::Daemon;
use crate::serve::reply;

/// The image's `/DIVERGE.md`, as the daemon can reach it.
///
/// The rules, in order:
///
/// 1. Every container the daemon runs whose image's digest is the
///    request's is a candidate — an agent's, or a tool's the daemon runs
///    itself; a connected tool is another daemon's container and has no
///    info channel here. The digest is the image, so the references are
///    not compared. The candidates are not narrowed by the asker's
///    grants over agents or tools: the answer says nothing of the
///    container, only of the image the asker named.
/// 2. With any candidate, one is chosen at random and its run's `info`
///    channel asked: its document is `Info`, none shipped is `Missing`,
///    a failure is `Error`. No other candidate and no provider is asked
///    after a failure.
/// 3. With none, a connected provider is chosen at random among those
///    that pull from a registry one of the image's references names —
///    the test a run's choice makes, [`capable::registry`]; with none
///    of those, the provider [`locate`] finds holding the image among
///    the daemon's own built images. With no provider either way, the
///    answer is `NoSource`, and nothing is asked.
/// 4. The provider is asked
///    [`containers::info`](diverge_sdk::provider::endpoints::containers::info),
///    which reads the file out of the image and runs nothing: its
///    document is `Info`, none shipped is `Missing`, the image found
///    nowhere is `NoSource`, and a failure is `Error`, in the
///    provider's words where it said any. Nothing is retried.
pub async fn info(daemon: &Arc<Daemon>, image: &Image) -> Frame {
    if let Some(answer) = from_running(daemon, image).await {
        return answer;
    }
    let Some(provider) = choose_provider(daemon, image).await else {
        return Frame::NoSource;
    };
    let Some(handle) = daemon.live.provider(&provider).await else {
        return Frame::Error(reply::failure(&"the provider chosen is not connected now"));
    };
    let asked = request::Frame { image: image.clone() };
    match execute::execute(&handle, &asked).await {
        Ok(found) => document(found),
        Err(execute::ExecuteError::NoSource) => Frame::NoSource,
        Err(execute::ExecuteError::Provider(error)) => Frame::Error(error),
        Err(error) => Frame::Error(reply::failure(&error)),
    }
}

/// Rules 1 and 2: the answer of a running container of the image, or
/// `None` when none runs.
async fn from_running(daemon: &Daemon, image: &Image) -> Option<Frame> {
    let agents: Vec<_> = daemon
        .live
        .agent_runs()
        .await
        .into_iter()
        .filter(|run| matches!(&run.attestation, attestation::Caller::Agent(agent) if agent.image.digest == image.digest))
        .collect();
    let tools: Vec<_> = daemon
        .live
        .tool_runs()
        .await
        .into_iter()
        .filter(|run| matches!(run.handle, ToolHandle::Run(_)) && run.attestation.image().digest == image.digest)
        .collect();
    let count = agents.len() + tools.len();
    if count == 0 {
        return None;
    }
    let chosen = rand::rng().random_range(0..count);
    let read = match chosen.checked_sub(agents.len()) {
        None => agents[chosen].handle.info().await,
        Some(tool) => match &tools[tool].handle {
            ToolHandle::Run(handle) => handle.info().await,
            ToolHandle::Connected(_) => return Some(Frame::Error(reply::failure(&"a connected tool has no info channel here"))),
        },
    };
    Some(match read {
        Ok(found) => document(found),
        Err(UnaryError::Refused(error)) => Frame::Error(error),
        Err(error) => Frame::Error(reply::failure(&error)),
    })
}

/// Rule 3: the provider to ask, or `None` when no connected provider
/// can obtain the image.
async fn choose_provider(daemon: &Arc<Daemon>, image: &Image) -> Option<Identity> {
    let mut pulling = Vec::new();
    for identity in daemon.live.connected_providers().await {
        let pulls = daemon
            .live
            .capabilities(&identity)
            .await
            .is_some_and(|info| capable::registry(&info, image).is_ok());
        if pulls {
            pulling.push(identity);
        }
    }
    if let Some(chosen) = pulling.choose(&mut rand::rng()) {
        return Some(chosen.clone());
    }
    locate(daemon, &image.digest, None).await
}

/// A document read, or none shipped, as the answer.
fn document(found: Option<Document>) -> Frame {
    match found {
        Some(document) => Frame::Info(document),
        None => Frame::Missing,
    }
}
