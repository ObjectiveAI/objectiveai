//! One incoming credential, as the store holds it.

use std::net::IpAddr;

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::daemon::endpoints::providers::incoming::Credential;
use diverge_sdk::daemon::endpoints::providers::incoming::list::server::response;

use crate::store::IncomingId;

/// An incoming credential row: what the wire reports, and the hash
/// of the key, which it never does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Incoming {
    /// The row.
    pub id: IncomingId,
    /// The identity a provider presenting the key has.
    pub identity: String,
    /// The one address the key is accepted from, if any.
    pub address: Option<IpAddr>,
    /// The SHA-256 of the key, hex.
    pub key_hash: String,
    /// Its tags, sorted bytewise.
    pub tags: Vec<String>,
    /// When it was added.
    pub created: DateTime<Utc>,
    /// Who added it.
    pub creator: Creator,
}

impl Incoming {
    /// The credential as the wire reports it, without the key.
    pub fn credential(&self) -> Credential {
        Credential {
            identity: self.identity.clone(),
            address: self.address,
        }
    }

    /// The provider's identity on the wire, once connected through
    /// the credential.
    pub fn provider(&self) -> Identity {
        Identity::IncomingUnbrokered {
            identity: self.identity.clone(),
        }
    }

    /// The credential as a list reports it, given whether a provider
    /// is connected through it now.
    pub fn report(&self, connected: bool) -> response::Incoming {
        response::Incoming {
            credential: self.credential(),
            connected,
            created: self.created,
            creator: self.creator.clone(),
            tags: self.tags.clone(),
        }
    }
}
