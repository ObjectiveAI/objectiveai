//! The providers connected now, one connection per identity.

use std::collections::HashMap;
use std::sync::Arc;

use diverge_sdk::daemon::agent_log::Identity;
use diverge_sdk::daemon::endpoints::providers::Status;
use diverge_sdk::provider::endpoints::info::server::response::Frame as Info;
use diverge_sdk::wire::client::handle::Handle;

/// Every provider the daemon holds a connection to, one slot per
/// identity. No two connections hold one `(issuer, subject)`, whichever
/// end dialled: the handshake holds the identity, and refuses a second
/// connection under it — see
/// [`DaemonAuthenticator`](crate::judge::DaemonAuthenticator) — so a
/// slot is never asked for twice.
#[derive(Default)]
pub struct Peers {
    /// One slot per identity connected, or being admitted.
    pub slots: HashMap<Identity, Slot>,
}

/// One provider's connection: where the daemon stands with it.
pub struct Slot {
    /// Reserved, connected, or a different version: see
    /// [`Standing`].
    pub standing: Standing,
}

/// Where the daemon stands with the provider in a slot.
///
/// A slot is RESERVED at the socket, so that no second socket of the
/// identity is admitted meanwhile, and FILLED once the provider has
/// answered the daemon's `info` ask: with the handle and the answer,
/// when the provider's version is the daemon's — the one standing in
/// which the provider is used for anything — or with the handle kept
/// and the answer, when it is not: the socket is held open, so the
/// provider is not dialled again and again, and never used. A
/// reserved slot reads as disconnected to everyone but the attach
/// filling it.
pub enum Standing {
    /// The socket is open and the provider is being asked its info.
    Pending,
    /// The provider answered its info with the daemon's version, and
    /// this is the handle every request to it rides, with what it
    /// answered: its capabilities, which the daemon asks nothing
    /// outside of.
    Connected {
        /// The caller's handle on the provider.
        handle: Handle,
        /// The provider's version and capabilities, as answered.
        info: Arc<Info>,
    },
    /// The provider answered its info with a version that is not the
    /// daemon's: the handle is kept so the socket stays open, and
    /// handed to nobody.
    DifferentVersion {
        /// The handle, holding the socket open: read by nothing, kept
        /// so the sink lives with the slot.
        _handle: Handle,
        /// The provider's version and capabilities, as answered.
        info: Arc<Info>,
    },
}

impl Peers {
    /// Reserve the slot for `identity`, the handshake holding it: the
    /// slot pending, and the caller's to fill and to give back.
    pub fn reserve(&mut self, identity: Identity) {
        self.slots.insert(identity, Slot { standing: Standing::Pending });
    }

    /// The slot filled with where the daemon stands now; `false` when
    /// no slot is the identity's.
    pub fn fill(&mut self, identity: &Identity, standing: Standing) -> bool {
        match self.slots.get_mut(identity) {
            Some(slot) => {
                slot.standing = standing;
                true
            }
            None => false,
        }
    }

    /// The slot given back; `false` when there was none to give.
    pub fn release(&mut self, identity: &Identity) -> bool {
        self.slots.remove(identity).is_some()
    }

    /// The standing of the identity's slot as a list reports it: a
    /// pending slot is as good as none.
    pub fn status(&self, identity: &Identity) -> Status {
        match self.slots.get(identity).map(|slot| &slot.standing) {
            Some(Standing::Connected { info, .. }) => Status::Connected { info: (**info).clone() },
            Some(Standing::DifferentVersion { info, .. }) => Status::DifferentVersion { info: (**info).clone() },
            Some(Standing::Pending) | None => Status::Disconnected,
        }
    }
}
