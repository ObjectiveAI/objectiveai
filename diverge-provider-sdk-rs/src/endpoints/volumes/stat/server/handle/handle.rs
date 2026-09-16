//! Answering a stat, from a scope and a manager.

use super::super::response;
use crate::encode::{Encode, Writer};
use crate::endpoints::volumes::refusal;
use crate::endpoints::volumes::stat::client::request;
use crate::server::scope_handle::ScopeHandle;
use crate::server::volume::Volume as _;
use crate::server::volume_manager::VolumeManager;
use crate::shared::error::Error;

/// Examine the volume and end the scope.
///
/// One question, one answer, done — so this consumes the scope rather
/// than handing it back. There is nothing a provider does with a
/// stat's scope afterwards.
///
/// # Under the lock
///
/// The volume is [`got`](VolumeManager::get) and then
/// [`locked`](crate::server::volume::Volume::lock) for the length of
/// the examination, so what is reported is the volume at rest, with
/// no container writing to it. A lock that is held — the volume is
/// mounted in a running container, or under another request — is the
/// stat refused with [`refusal::mounted`]: the lock never waits, and
/// the endpoint has no frame for it but the error. The lock is given
/// back whatever the examination answered.
///
/// # Every failure becomes a frame
///
/// A name the caller has no volume by is [`refusal::unknown`]; a
/// manager or a volume that will not answer is its error, flattened.
/// The endpoint has one place to put a failure, so there is nothing
/// else to say.
///
/// A response that will not ENCODE is the one failure with nowhere to
/// go, and the scope simply finishes without an answer. A caller reads
/// that as a provider that said nothing, which is what happened.
///
/// # The request arrives decoded
///
/// [`server::handle`](crate::server::handle::handle) reads every
/// request once to dispatch it, and hands the result here — so a
/// malformed request never reaches this function, and nothing in it
/// decodes one.
pub async fn handle<M>(
    scope: ScopeHandle,
    request: request::Frame,
    client_identity: &str,
    manager: &M,
) where
    M: VolumeManager,
    M::Error: Into<Error>,
{
    let frame = match stat(manager, client_identity, &request.name).await {
        Ok(stat) => response::Frame::Stat(stat),
        Err(error) => response::Frame::Error(error),
    };

    let mut buffer = Vec::new();
    if frame.encode(&mut Writer::new(&mut buffer)).is_ok() {
        scope.send_response(&buffer).await;
    }
    scope.send_response_finish().await;
}

/// The volume found, locked, examined, and unlocked; or the one error
/// the endpoint answers with, whichever step it came from.
async fn stat<M>(
    manager: &M,
    client_identity: &str,
    name: &str,
) -> Result<response::Stat, Error>
where
    M: VolumeManager,
    M::Error: Into<Error>,
{
    let volume = manager
        .get(client_identity, name)
        .await
        .map_err(Into::into)?
        .ok_or_else(|| refusal::unknown(name))?;
    if !volume.lock() {
        return Err(refusal::mounted(name));
    }
    let stat = volume.stat().await;
    volume.unlock();
    stat.map_err(Into::into)
}
