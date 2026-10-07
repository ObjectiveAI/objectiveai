//! One outgoing provider, as the store holds it.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::daemon::endpoints::providers::outgoing::list::server::response;
use diverge_sdk::daemon::endpoints::providers::outgoing::{Kind, Mode};

use crate::store::OutgoingId;

/// An outgoing provider row: what the wire reports, and the mode
/// whole, which it never does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outgoing {
    /// The row.
    pub id: OutgoingId,
    /// The address, as the add gave it: the identity.
    pub address: String,
    /// How the daemon authenticates there, credential and all.
    pub mode: Mode,
    /// When the daemon's connection last opened or closed; absent
    /// for one never dialled.
    pub last_connected: Option<DateTime<Utc>>,
    /// When it was added.
    pub created: DateTime<Utc>,
    /// Who added it.
    pub creator: Creator,
}

impl Outgoing {
    /// The mode's name, which is what the wire reports of it.
    pub fn kind(&self) -> Kind {
        match self.mode {
            Mode::Unbrokered { .. } => Kind::Unbrokered,
        }
    }

    /// The provider's identity on the wire: its address.
    pub fn identity(&self) -> Identity {
        Identity::Outgoing {
            address: self.address.clone(),
        }
    }

    /// The provider as a list reports it, given whether the daemon
    /// holds a connection to it now.
    pub fn report(&self, connected: bool) -> response::Outgoing {
        response::Outgoing {
            address: self.address.clone(),
            kind: self.kind(),
            connected,
            last_connected: self.last_connected,
            created: self.created,
            creator: self.creator.clone(),
        }
    }
}
