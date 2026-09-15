//! The volumes a run has locked, and the giving back.

use std::mem;

use crate::endpoints::volumes::refusal;
use crate::server::volume::Volume;
use crate::server::volume_manager::VolumeManager;
use crate::shared::error::Error;

/// Every volume a run's request names, locked, from before anything
/// is fetched or deployed until the run ends.
///
/// The server half's whole enforcement of one container per volume:
/// the lock on each volume — see [`Volume::lock`] — is what
/// "mounted in a running container" IS, so taking every lock is
/// accepting the request and giving every one back is the run over.
///
/// # Every ending gives them back
///
/// [`release`](Self::release) is called on the two paths the run
/// handler ends by — the preparation failing before the id is out,
/// and the one teardown every finished run reaches, whether it was
/// stopped, its container left, or its caller went away. A run
/// future dropped before either — the runtime going down mid-await —
/// reaches neither, and the [`Drop`] here spawns the unlocks it can
/// no longer await, on the runtime it is on, so a lock is never left
/// held by a run that no longer exists. Nothing is ever locked twice
/// and nothing is ever unlocked twice: what `release` takes out of
/// the list, `Drop` does not see.
pub(crate) struct Held<V: Volume + 'static> {
    volumes: Vec<V>,
}

/// Why a request's volumes could not all be taken.
pub(crate) enum Refused {
    /// A volume the request names has its lock held — mounted in
    /// another container of the caller's, or under a request in
    /// flight — and this is its name. The run's own
    /// [`VolumeMounted`](crate::shared::containers::response::VolumeMounted).
    Mounted(String),
    /// A volume the request names is not the caller's, or the
    /// provider could not answer for one: the run's error.
    Error(Error),
}

impl<V: Volume + 'static> Held<V> {
    /// Lock every volume `names` names, in order, or none of them.
    ///
    /// Each name is [`got`](VolumeManager::get) and
    /// [`locked`](Volume::lock) in turn — in turn, not at once,
    /// because a name listed twice must refuse on its second
    /// appearance and a refusal must give back exactly what was
    /// taken before it. A name the caller has no volume by is
    /// [`Refused::Error`] with [`refusal::unknown`]; a lock that is
    /// held is [`Refused::Mounted`]; a provider that will not answer
    /// is [`Refused::Error`] with its error. On any of them, every
    /// lock taken so far is given back before the refusal is
    /// returned.
    pub async fn take<'a, M>(
        manager: &M,
        client_identity: &str,
        names: impl IntoIterator<Item = &'a str>,
    ) -> Result<Self, Refused>
    where
        M: VolumeManager<Volume = V>,
        M::Error: Into<Error>,
        V::Error: Into<Error>,
    {
        let mut held = Held { volumes: Vec::new() };
        for name in names {
            match held.take_one(manager, client_identity, name).await {
                Ok(()) => {}
                Err(refused) => {
                    held.release().await;
                    return Err(refused);
                }
            }
        }
        Ok(held)
    }

    /// One name: found, locked, and kept.
    async fn take_one<M>(
        &mut self,
        manager: &M,
        client_identity: &str,
        name: &str,
    ) -> Result<(), Refused>
    where
        M: VolumeManager<Volume = V>,
        M::Error: Into<Error>,
        V::Error: Into<Error>,
    {
        let volume = manager
            .get(client_identity, name)
            .await
            .map_err(|error| Refused::Error(error.into()))?
            .ok_or_else(|| Refused::Error(refusal::unknown(name)))?;
        match volume.lock().await {
            Ok(true) => {
                self.volumes.push(volume);
                Ok(())
            }
            Ok(false) => Err(Refused::Mounted(name.to_string())),
            Err(error) => Err(Refused::Error(error.into())),
        }
    }

    /// Give every lock back, awaited, in the order they were taken.
    ///
    /// A provider that cannot unlock one has nothing this run can do
    /// about it, and the run is ending either way, so the failure is
    /// not reported: the next request on that volume is what will
    /// see it.
    pub async fn release(&mut self) {
        for volume in mem::take(&mut self.volumes) {
            let _ = volume.unlock().await;
        }
    }
}

/// The unlocks a dropped run could not await, spawned instead — on
/// the runtime this is dropped on, when there is one. Empty after a
/// [`release`](Held::release), so a run that ended in order spawns
/// nothing.
impl<V: Volume + 'static> Drop for Held<V> {
    fn drop(&mut self) {
        if self.volumes.is_empty() {
            return;
        }
        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            return;
        };
        for volume in mem::take(&mut self.volumes) {
            runtime.spawn(async move {
                let _ = volume.unlock().await;
            });
        }
    }
}
