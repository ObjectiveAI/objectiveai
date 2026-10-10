//! One daemon record, as the store holds it.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::providers::daemons::Link;
use diverge_sdk::daemon::endpoints::providers::daemons::list::server::response;
use diverge_sdk::daemon::endpoints::providers::outgoing::{Kind, Mode};

use crate::store::DaemonId;

/// A daemon record row: what the wire reports, and the mode whole,
/// which it never does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// The row.
    pub id: DaemonId,
    /// The name, as the add gave it: the daemon's identity here.
    pub name: String,
    /// How this daemon authenticates there, credential and all.
    pub mode: Mode,
    /// The providers it is reached through, each with the identity it
    /// is known by there.
    pub links: Vec<Link>,
    /// Its tags, sorted bytewise.
    pub tags: Vec<String>,
    /// When it was added.
    pub created: DateTime<Utc>,
    /// Who added it.
    pub creator: Creator,
}

impl Record {
    /// The mode's name, which is what the wire reports of it.
    pub fn kind(&self) -> Kind {
        match self.mode {
            Mode::Unbrokered { .. } => Kind::Unbrokered,
        }
    }

    /// The daemon as a list reports it, given whether this daemon
    /// holds a connection to it now.
    pub fn report(&self, connected: bool) -> response::Daemon {
        response::Daemon {
            name: self.name.clone(),
            kind: self.kind(),
            links: self.links.clone(),
            connected,
            created: self.created,
            creator: self.creator.clone(),
            tags: self.tags.clone(),
        }
    }
}
