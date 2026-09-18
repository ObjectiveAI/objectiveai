//! The volumes a run holds, shared, and the giving back.

use std::sync::Arc;

use futures_util::future;

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
    /// Every name is [`got`](VolumeManager::get) beside every other —
    /// a lookup may run a provider's hook, a process, and three of
    /// them one after another would be three processes' latency end
    /// to end — and then each found volume is [`mounted`](Volume::mount)
    /// in request order, one after another, since a hold is one
    /// atomic that answers at once and taking them in order is what
    /// lets a refusal give back exactly what was taken before it. A
    /// name the caller has no volume by is [`Refused::Error`] with
    /// [`refusal::unknown`]; a provider that will not answer a lookup
    /// is [`Refused::Error`] with its error; both before anything is
    /// held. A volume held exclusively is [`Refused::Held`], with the
    /// holds taken before it given back.
    pub async fn take<'a, M>(
        manager: &M,
        client_identity: &str,
        names: impl IntoIterator<Item = &'a str>,
    ) -> Result<Self, Refused>
    where
        M: VolumeManager<Volume = V>,
        M::Error: Into<Error>,
    {
        let names: Vec<&str> = names.into_iter().collect();
        let found = future::join_all(names.iter().map(|name| async move {
            match manager.get(client_identity, name).await {
                Ok(Some(volume)) => Ok(volume),
                Ok(None) => Err(Refused::Error(refusal::unknown(name))),
                Err(error) => Err(Refused::Error(error.into())),
            }
        }))
        .await;
        let mut volumes = Vec::with_capacity(found.len());
        for outcome in found {
            volumes.push(outcome?);
        }
        let mut held = Held {
            volumes: Vec::with_capacity(volumes.len()),
        };
        for (name, volume) in names.iter().zip(volumes) {
            if !volume.mount().await {
                held.give_back().await;
                return Err(Refused::Held(name.to_string()));
            }
            held.volumes.push(Arc::new(volume));
        }
        Ok(held)
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
