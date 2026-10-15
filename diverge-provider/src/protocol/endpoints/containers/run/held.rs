//! The volumes a run holds, shared, and the giving back.

use std::collections::HashMap;
use std::sync::Arc;

use futures_util::future;

use crate::protocol::hold::{self, Allow, Holder};
use crate::protocol::volume::Volume;
use crate::protocol::volume_manager::VolumeManager;
use diverge_sdk::provider::endpoints::info::server::response::Capabilities;
use diverge_sdk::provider::endpoints::volumes::Mode;
use diverge_sdk::provider::endpoints::volumes::refusal;
use diverge_sdk::shared::containers::response::VolumeMode;
use diverge_sdk::shared::error::Error;

/// Every volume a run's request names, held shared, from before
/// anything is fetched or deployed until the run ends.
///
/// The server half's whole enforcement of the rule that nothing
/// examines, resizes or deletes a volume while a container has it:
/// the shared hold on each volume — see [`Volume::mount`] — is what
/// "mounted in a running container" IS, so taking every hold is
/// accepting the request and giving every one back is the run over.
/// One run is one holder of a volume: a run naming one volume at
/// several paths holds it once, so that a mode the provider mounts
/// once is mounted at two paths of one container while a second
/// container is refused, and a count of holders counts containers.
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
    /// One per distinct name, in the order first named: the holds.
    held: Vec<Arc<V>>,
    /// One per entry of the request's mounts, in request order, a
    /// name repeated sharing its volume. Shared, so a watch of one
    /// may keep a handle for the run's life beside this.
    volumes: Vec<Arc<V>>,
}

/// Why a request's volumes could not all be taken: each the run's own
/// response, naming the volume.
pub(crate) enum Refused {
    /// A volume the request names is under a stat, a read, a write,
    /// a filetree, an edit or a delete. The run's `VolumeHeld`.
    Held(String),
    /// A running container has a volume the request names, and the
    /// provider mounts a volume of its mode once. The run's
    /// `VolumeMounted`.
    Mounted(String),
    /// A FUSE serve has a volume the request names, and the provider
    /// does not mount a volume of its mode beside one. The run's
    /// `VolumeFuseServed`.
    FuseServed(String),
    /// The provider does not mount a volume of the mode a volume the
    /// request names is in. The run's `VolumeMountUnsupported`.
    MountUnsupported(String),
    /// A volume the request names is not in the mode its mount means
    /// it to have — or two mounts name it under two modes, one of
    /// which it is not in — and this is its name and the mode it is
    /// in. The run's `VolumeMode`.
    Mode(VolumeMode),
    /// A volume the request names is not the caller's, or the
    /// provider could not answer for one: the run's error.
    Error(Error),
}

impl<V: Volume> Held<V> {
    /// Hold every volume `mounts` names, once each, or none of them,
    /// each in the mode its mounts mean and as the provider's
    /// `capabilities` for that mode allow.
    ///
    /// Every distinct name is [`got`](VolumeManager::get) beside
    /// every other — a lookup may run a provider's hook, a process,
    /// and three of them one after another would be three processes'
    /// latency end to end — and then each found volume is
    /// [`mounted`](Volume::mount) in the order first named, one after
    /// another, since a hold is one atomic that answers at once and
    /// taking them in order is what lets a refusal give back exactly
    /// what was taken before it. A name the caller has no volume by
    /// is [`Refused::Error`] with [`refusal::unknown`]; a provider
    /// that will not answer a lookup is [`Refused::Error`] with its
    /// error; both before anything is held. The mode each volume is
    /// in is judged under its own lock, through the `admit` the
    /// mount is given: a mode some mount naming the volume does not
    /// mean is declined, [`Refused::Mode`]; a mode the capabilities
    /// do not mount is declined, [`Refused::MountUnsupported`]; and
    /// a volume a party the capabilities do not allow beside holds
    /// is [`Refused::Held`], [`Refused::Mounted`] or
    /// [`Refused::FuseServed`], as [`hold`] says — each with the
    /// holds taken before it given back.
    pub async fn take<'a, M>(
        manager: &M,
        client_identity: &str,
        mounts: impl IntoIterator<Item = (&'a str, Mode)>,
        capabilities: &Capabilities,
    ) -> Result<Self, Refused>
    where
        M: VolumeManager<Volume = V>,
        M::Error: Into<Error>,
    {
        let mounts: Vec<(&str, Mode)> = mounts.into_iter().collect();
        let names: Vec<&str> = mounts.iter().map(|(name, _)| *name).collect();
        let mut distinct: Vec<&str> = Vec::with_capacity(names.len());
        for name in &names {
            if !distinct.contains(name) {
                distinct.push(name);
            }
        }
        let found = future::join_all(distinct.iter().map(|name| async move {
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
            held: Vec::with_capacity(volumes.len()),
            volumes: Vec::with_capacity(names.len()),
        };
        let mut by_name: HashMap<&str, Arc<V>> = HashMap::with_capacity(distinct.len());
        for (name, volume) in distinct.iter().zip(volumes) {
            let meant: Vec<Mode> = mounts
                .iter()
                .filter(|(named, _)| named == name)
                .map(|(_, mode)| *mode)
                .collect();
            let admit = |mode: Mode| -> Option<Allow> {
                if meant.iter().any(|m| *m != mode) {
                    return None;
                }
                let allowed = capabilities.volume(mode)?;
                if !allowed.mount {
                    return None;
                }
                Some(Allow {
                    many: allowed.mount_many,
                    beside: allowed.mount_and_fuse_serve,
                })
            };
            let refused = match volume.mount(Holder::Run, admit).await {
                Ok(_) => None,
                Err(hold::Refused::Declined(mode)) if meant.iter().any(|m| *m != mode) => {
                    Some(Refused::Mode(VolumeMode {
                        name: name.to_string(),
                        mode,
                    }))
                }
                Err(hold::Refused::Declined(_)) => {
                    Some(Refused::MountUnsupported(name.to_string()))
                }
                Err(hold::Refused::Held(hold::Held::Locked)) => {
                    Some(Refused::Held(name.to_string()))
                }
                Err(hold::Refused::Held(hold::Held::Mounted)) => {
                    Some(Refused::Mounted(name.to_string()))
                }
                Err(hold::Refused::Held(hold::Held::FuseServed)) => {
                    Some(Refused::FuseServed(name.to_string()))
                }
            };
            if let Some(refused) = refused {
                held.give_back().await;
                return Err(refused);
            }
            let volume = Arc::new(volume);
            held.held.push(Arc::clone(&volume));
            by_name.insert(name, volume);
        }
        held.volumes = names
            .iter()
            .map(|name| Arc::clone(&by_name[name]))
            .collect();
        Ok(held)
    }

    /// The volumes, in the order the request names them: one per
    /// entry of `volume_mounts`, a volume named twice appearing
    /// twice, as one shared handle.
    pub(crate) fn volumes(&self) -> &[Arc<V>] {
        &self.volumes
    }

    /// Every hold given back, in the order taken: the run is over.
    pub(crate) async fn give_back(self) {
        for volume in &self.held {
            volume.unmount(Holder::Run).await;
        }
    }
}
