//! The volumes a run has locked, and the giving back.

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
/// # Dropping it gives them back
///
/// [`Drop`] unlocks every volume, in the order they were taken, and
/// that is the only way they are given back — so every ending of a
/// run unlocks, the ones the handler wrote and the one it did not:
/// the preparation failing before the id is out, the teardown every
/// finished run reaches whether it was stopped, its container left,
/// or its caller went away, and a run future dropped mid-await. The
/// handler drops it where the teardown wants the unlock to fall,
/// after the container is stopped. Nothing is ever locked twice and
/// nothing is ever unlocked twice.
pub(crate) struct Held<V: Volume> {
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

impl<V: Volume> Held<V> {
    /// Lock every volume `names` names, in order, or none of them.
    ///
    /// Each name is [`got`](VolumeManager::get) and
    /// [`locked`](Volume::lock) in turn — in turn, not at once,
    /// because a name listed twice must refuse on its second
    /// appearance and a refusal must give back exactly what was
    /// taken before it. A name the caller has no volume by is
    /// [`Refused::Error`] with [`refusal::unknown`]; a lock that is
    /// held is [`Refused::Mounted`]; a provider that will not answer
    /// the lookup is [`Refused::Error`] with its error. On any of
    /// them, the locks taken so far are given back — the half-built
    /// `Held` is dropped — before the refusal is returned.
    pub async fn take<'a, M>(
        manager: &M,
        client_identity: &str,
        names: impl IntoIterator<Item = &'a str>,
    ) -> Result<Self, Refused>
    where
        M: VolumeManager<Volume = V>,
        M::Error: Into<Error>,
    {
        let mut held = Held { volumes: Vec::new() };
        for name in names {
            held.take_one(manager, client_identity, name).await?;
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
    {
        let volume = manager
            .get(client_identity, name)
            .await
            .map_err(|error| Refused::Error(error.into()))?
            .ok_or_else(|| Refused::Error(refusal::unknown(name)))?;
        if !volume.lock() {
            return Err(Refused::Mounted(name.to_string()));
        }
        self.volumes.push(volume);
        Ok(())
    }
}

impl<V: Volume> Drop for Held<V> {
    fn drop(&mut self) {
        for volume in &self.volumes {
            volume.unlock();
        }
    }
}
