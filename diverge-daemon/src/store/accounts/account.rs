//! One account, as the store holds it.

use std::net::IpAddr;

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::accounts::list::server::response;
use diverge_sdk::daemon::endpoints::accounts::{Credential, Reference};

use crate::store::AccountId;

/// An account row with its roles: what the wire reports and what it
/// never does — the id, and the hash of the key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    /// The row.
    pub id: AccountId,
    /// The name, if the account has one.
    pub name: Option<String>,
    /// The credential's identity, if the account has a credential.
    pub identity: Option<String>,
    /// The credential's address, if it names one.
    pub address: Option<IpAddr>,
    /// The SHA-256 of the key, hex, if the account has a credential.
    pub key_hash: Option<String>,
    /// What the account is for.
    pub description: Option<String>,
    /// The roles it holds, by name, sorted bytewise.
    pub roles: Vec<String>,
    /// Its tags, sorted bytewise.
    pub tags: Vec<String>,
    /// When it was made.
    pub created: DateTime<Utc>,
    /// Who made it.
    pub creator: Creator,
}

impl Account {
    /// The identity the account is served under: its name when it has
    /// one, and otherwise the identity its credential names. An
    /// account has one or the other.
    pub fn identity(&self) -> &str {
        self.name.as_deref().or(self.identity.as_deref()).unwrap_or_default()
    }

    /// How the wire names it: by name when it has one, else by
    /// identity.
    pub fn reference(&self) -> Reference {
        match (&self.name, &self.identity) {
            (Some(name), _) => Reference::Name { name: name.clone() },
            (None, identity) => Reference::Identity {
                identity: identity.clone().unwrap_or_default(),
            },
        }
    }

    /// The credential as the wire reports it, without the key, if the
    /// account has one.
    pub fn credential(&self) -> Option<Credential> {
        self.identity.as_ref().map(|identity| Credential {
            identity: identity.clone(),
            address: self.address,
        })
    }

    /// The account as a list reports it, given whether a client is
    /// connected as it now.
    pub fn report(&self, connected: bool) -> response::Account {
        response::Account {
            name: self.name.clone(),
            credential: self.credential(),
            description: self.description.clone(),
            roles: self.roles.clone(),
            connected,
            tags: self.tags.clone(),
            created: self.created,
            creator: self.creator.clone(),
        }
    }
}
