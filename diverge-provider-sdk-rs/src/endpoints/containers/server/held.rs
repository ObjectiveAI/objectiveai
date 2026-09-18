//! The volumes a run holds, shared, and the giving back.

use std::sync::Arc;

use crate::endpoints::volumes::refusal;
use crate::server::volume::Volume;
use crate::server::volume_manager::VolumeManager;
use crate::shared::error::Error;

/// Every volume a run's request names, held shared, from before
/// anything is fetched or deployed until the run ends.
///
/// The server half's whole enforcement of the rule that nothing
/// examines, resizes or deletes a volume while a container has it:
/// the shared hold on each volume — see [`Volume::mount`] — is what
/// "mounted in a running container" IS, so taking every hold is
/// accepting the request and giving every one back is the run over.
/// Any number of runs may hold one volume at once; a run naming one
/// volume twice holds it twice.
///
/// # Giving back is explicit
///
/// [`give_back`](Self::give_back) unmounts every volume, in the
/// order they were taken, and the handler calls it at every ending
/// it writes — the preparation failing before the id is out, and the
/// teardown every finished run reaches whether it was stopped, its
/// container left, or its caller went away — after the container is
/// stopped, which is where the teardown wants it. It is a call and
/// not a `Drop` because giving back is a future, and the handler's
/// futures are never dropped mid-await: the session drains every
/// scope before it goes. Nothing is ever taken twice for one name
/// and nothing is ever given back twice.
pub(crate) struct Held<V: Volume> {
    /// Shared, so a watch of one may keep a handle for the run's life
    /// beside this.
    volumes: Vec<Arc<V>>,
}

/// Why a request's volumes could not all be taken.
pub(crate) enum Refused {
    /// A volume the request names is held exclusively — under a stat,
    /// an edit or a delete in flight — and this is its name. The
    /// run's own
    /// [`VolumeHeld`](crate::shared::containers::response::VolumeHeld).
    Held(String),
    /// A volume the request names is not the caller's, or the
    /// provider could not answer for one: the run's error.
    Error(Error),
}

impl<V: Volume> Held<V> {
    /// Hold every volume `names` names, in order, or none of them.
    ///
    /// Each name is [`got`](VolumeManager::get) and
    /// [`mounted`](Volume::mount) in turn — in turn, not at once, so
    /// a refusal gives back exactly what was taken before it. A name
    /// the caller has no volume by is [`Refused::Error`] with
    /// [`refusal::unknown`]; a volume held exclusively is
    /// [`Refused::Held`]; a provider that will not answer the lookup
    /// is [`Refused::Error`] with its error. On any of them, the holds
    /// taken so far are given back before the refusal is returned.
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
            if let Err(refused) = held.take_one(manager, client_identity, name).await {
                held.give_back().await;
                return Err(refused);
            }
        }
        Ok(held)
    }

    /// One name: found, held shared, and kept.
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
        if !volume.mount().await {
            return Err(Refused::Held(name.to_string()));
        }
        self.volumes.push(Arc::new(volume));
        Ok(())
    }

    /// The volumes, in the order the request names them: one per
    /// entry of `volume_mounts`, a volume named twice appearing
    /// twice.
    pub(crate) fn volumes(&self) -> &[Arc<V>] {
        &self.volumes
    }

    /// Every hold given back, in the order taken: the run is over.
    pub(crate) async fn give_back(self) {
        for volume in &self.volumes {
            volume.unmount().await;
        }
    }
}
