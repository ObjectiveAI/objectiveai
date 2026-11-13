//! An image's DIVERGE.md: from the daemon's record, from a container
//! of it running, or from whichever provider or remote daemon holds
//! it, asked all at once.

use std::sync::Arc;

use diverge_sdk::daemon::agent_log::Identity;
use diverge_sdk::daemon::endpoints::daemons::DaemonId;
use diverge_sdk::daemon::endpoints::images::info::client::{execute as daemon_execute, request as daemon_request};
use diverge_sdk::daemon::endpoints::images::info::server::response::Frame;
use diverge_sdk::provider::endpoints::containers::client::UnaryError;
use diverge_sdk::provider::endpoints::containers::info::client::{execute, request};
use diverge_sdk::shared::containers::info::Document;
use diverge_sdk::shared::containers::request::Image;
use diverge_sdk::shared::mcp::attestation;
use futures_util::StreamExt as _;
use futures_util::stream::FuturesUnordered;
use rand::Rng as _;

use super::{Holder, Reach, asks, cancel_rest};
use crate::containers::run::ToolHandle;
use crate::daemon::Daemon;
use crate::daemons;
use crate::serve::reply;
use crate::store;

/// The image's `/DIVERGE.md`, as the daemon can reach it.
///
/// The rules, in order:
///
/// 1. The daemon's record of the digest
///    ([`store::image_info`]) is read first. A document on it is the
///    answer, `Info`, and nothing is asked: the digest is the image,
///    so what it ships does not change.
/// 2. Every container the daemon runs whose image's digest is the
///    request's is a candidate — an agent's, or a tool's the daemon runs
///    itself; a connected tool is another daemon's container and has no
///    info channel here. The digest is the image, so the references are
///    not compared. The candidates are not narrowed by the asker's
///    grants over agents or tools: the answer says nothing of the
///    container, only of the image the asker named.
/// 3. With any candidate, one is chosen at random and its run's `info`
///    channel asked: its document is `Info`, none shipped is `Missing`,
///    a failure is `Error`. No other candidate and no provider is asked
///    after a failure.
/// 4. With none, who holds the image is asked of everything in reach,
///    all at once, as [`locate`](super::locate) asks it: every
///    connected provider that can hold an image, and every remote
///    daemon recorded whose identity is neither in `ignore` nor this
///    daemon's own. It is the cheap question, an `images::oci` scope
///    opened, and nothing is pulled to answer it.
/// 5. Each that answers held is asked for the document, in the order
///    they answer, its scope cancelled first, since only the answer
///    was wanted: a provider by
///    [`containers::info`](diverge_sdk::provider::endpoints::containers::info),
///    which reads the file out of the image and runs nothing; a remote
///    daemon by its own `images info`, with `ignore` and this daemon's
///    identity, which it answers by these rules. An `Info` or a
///    `Missing` is the answer, and every ask still out is cancelled as
///    it answers. A holder that answers anything else is passed over,
///    and the next to answer held is asked.
/// 6. With no answer yet, every connected provider not asked under
///    rule 5 is asked `containers::info`, all at once, and the first
///    `Info` or `Missing` is the answer. A provider's held does not
///    look in its podman cache and its info does, so an image only a
///    cache holds — one built on the provider's host — is found here
///    and nowhere else; a provider that has no such image answers
///    without pulling anything.
/// 7. With no answer still, the answer is the last failure a holder
///    or a provider gave, in its words, and `NoSource` where none gave
///    any. A remote daemon's `Forbidden` is such a failure: it is the
///    remote's of this daemon, not this daemon's of the asker.
/// 8. An `Info` that rules 3 to 6 found is put on the record before
///    it is answered. Nothing else is: only a document is recorded,
///    so a `Missing`, a `NoSource` and an `Error` are each asked
///    again by a later ask.
///
/// Nothing times out. A record that cannot be read or written is the
/// `Err`, and no answer is given.
pub async fn info(daemon: &Arc<Daemon>, image: &Image, ignore: &[Identity]) -> Result<Frame, store::Error> {
    let recorded = {
        let mut conn = daemon.store.acquire().await?;
        store::image_info::by_digest(&mut conn, &image.digest).await?
    };
    if let Some(found) = recorded {
        return Ok(Frame::Info(found));
    }
    let answer = reach(daemon, image, ignore).await;
    if let Frame::Info(found) = &answer {
        let mut conn = daemon.store.acquire().await?;
        store::image_info::save(&mut conn, &image.digest, found).await?;
    }
    Ok(answer)
}

/// The image's document, or why there is none, in words: what a
/// template's creation and a dependency's declaration hold an image
/// to, so that every image the daemon deploys has a document on
/// record. The rules:
///
/// 1. The document is found as [`info`] finds it, record first, no
///    daemon ignored, and is on the record once this answers it.
/// 2. An image that ships no `/DIVERGE.md` has none.
/// 3. An image no source holds, or whose document could not be read
///    or does not parse, has none either, and the words say which.
///
/// A record that cannot be read or written is the outer `Err`.
pub async fn document(daemon: &Arc<Daemon>, image: &Image) -> Result<Result<Document, String>, store::Error> {
    Ok(match info(daemon, image, &[]).await? {
        Frame::Info(document) => Ok(document),
        Frame::Missing => Err("the image ships no /DIVERGE.md".to_string()),
        Frame::NoSource => Err("the image's document could not be read: no source holds the image".to_string()),
        Frame::Forbidden => Err("the image's document could not be read".to_string()),
        Frame::Error(error) => Err(format!("the image's document could not be read: {}", error.0)),
    })
}

/// Rules 2 to 7: the answer of a running container, else of a holder,
/// else of a provider.
async fn reach(daemon: &Arc<Daemon>, image: &Image, ignore: &[Identity]) -> Frame {
    if let Some(answer) = from_running(daemon, image).await {
        return answer;
    }
    // Rule 4.
    let mut asking = asks(daemon, image, &Reach::Full { except: None }, ignore).await;
    let mut asked = Vec::new();
    let mut last = None;
    // Rule 5.
    while let Some(answered) = asking.next().await {
        let Some(source) = answered else {
            continue;
        };
        let _ = source.scope.cancel().await;
        let answer = match &source.holder {
            Holder::Provider(identity) => {
                asked.push(identity.clone());
                from_provider(daemon, image, identity).await
            }
            Holder::Daemon(id) => from_daemon(daemon, image, ignore, *id).await,
        };
        if matches!(answer, Frame::Info(_) | Frame::Missing) {
            cancel_rest(asking);
            return answer;
        }
        if !matches!(answer, Frame::NoSource) {
            last = Some(answer);
        }
    }
    // Rule 6: all at once, since no provider's answer bears on
    // another's.
    let mut reading: FuturesUnordered<_> = daemon
        .live
        .connected_providers()
        .await
        .into_iter()
        .filter(|identity| !asked.contains(identity))
        .map(|identity| async move { from_provider(daemon, image, &identity).await })
        .collect();
    while let Some(answer) = reading.next().await {
        if matches!(answer, Frame::Info(_) | Frame::Missing) {
            return answer;
        }
        if !matches!(answer, Frame::NoSource) {
            last = Some(answer);
        }
    }
    // Rule 7.
    last.unwrap_or(Frame::NoSource)
}

/// One provider's answer to `containers::info`: its document, none
/// shipped, the image found nowhere, or a failure, in its words where
/// it said any.
async fn from_provider(daemon: &Daemon, image: &Image, provider: &Identity) -> Frame {
    let Some(handle) = daemon.live.provider(provider).await else {
        return Frame::Error(reply::failure(&"the provider is not connected now"));
    };
    let asked = request::Frame { image: image.clone() };
    match execute::execute(&handle, &asked).await {
        Ok(found) => shipped(found),
        Err(execute::ExecuteError::NoSource) => Frame::NoSource,
        Err(execute::ExecuteError::Provider(error)) => Frame::Error(error),
        Err(error) => Frame::Error(reply::failure(&error)),
    }
}

/// One remote daemon's answer to `images info`, asked with `ignore`
/// and this daemon's own identity so that no daemon is asked twice
/// along one path: its document, none shipped, no source, or a
/// failure — its `Forbidden` among them, rule 7.
async fn from_daemon(daemon: &Arc<Daemon>, image: &Image, ignore: &[Identity], id: DaemonId) -> Frame {
    let record = match daemon.store.acquire().await {
        Ok(mut conn) => store::daemons::by_id(&mut conn, id, false).await,
        Err(error) => Err(error),
    };
    let record = match record {
        Ok(Some(record)) => record,
        Ok(None) => return Frame::Error(reply::failure(&"the daemon that held the image is recorded no more")),
        Err(error) => return Frame::Error(reply::failure(&error)),
    };
    let Ok(peer) = daemons::connect(daemon, &record).await else {
        return Frame::Error(reply::failure(&"the daemon that held the image could not be reached"));
    };
    let mut passed: Vec<Identity> = ignore.to_vec();
    passed.extend(daemon.authenticator.own().cloned());
    let asked = daemon_request::Frame { image: image.clone(), ignore: passed, relay: None };
    match daemon_execute::execute(&peer.connected.handle, &asked).await {
        Ok(Frame::Forbidden) => Frame::Error(reply::failure(&"the daemon that held the image refused its document")),
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    }
}

/// Rules 2 and 3: the answer of a running container of the image, or
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
        Ok(found) => shipped(found),
        Err(UnaryError::Refused(error)) => Frame::Error(error),
        Err(error) => Frame::Error(reply::failure(&error)),
    })
}

/// A document read, or none shipped, as the answer.
fn shipped(found: Option<Document>) -> Frame {
    match found {
        Some(document) => Frame::Info(document),
        None => Frame::Missing,
    }
}
