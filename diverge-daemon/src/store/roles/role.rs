//! One role, as the store holds it.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::accounts::Reference;
use diverge_sdk::daemon::endpoints::roles::list::server::response;
use diverge_sdk::daemon::grant::Grant;

use crate::store::{AccountId, RoleId};

/// A role row with the accounts that hold it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Role {
    /// The row.
    pub id: RoleId,
    /// The name, which never changes.
    pub name: String,
    /// What the role is for.
    pub description: Option<String>,
    /// Its grants, in the order they were given.
    pub grants: Vec<Grant>,
    /// The accounts holding it, in no order.
    pub holders: Vec<Holder>,
    /// Its tags, sorted bytewise.
    pub tags: Vec<String>,
    /// When it was made.
    pub created: DateTime<Utc>,
    /// Who made it.
    pub creator: Creator,
}

/// An account holding a role: enough to name it either way the wire
/// does, so that a filter naming an account by identity finds a named
/// holder that also has that credential.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Holder {
    /// The account.
    pub id: AccountId,
    /// Its name, if any.
    pub name: Option<String>,
    /// Its credential's identity, if any.
    pub identity: Option<String>,
}

impl Holder {
    /// How the wire names the holder: by name when it has one, else
    /// by identity.
    pub fn reference(&self) -> Reference {
        match (&self.name, &self.identity) {
            (Some(name), _) => Reference::Name { name: name.clone() },
            (None, identity) => Reference::Identity {
                identity: identity.clone().unwrap_or_default(),
            },
        }
    }
}

impl Role {
    /// The role as a list reports it.
    pub fn report(&self) -> response::Role {
        response::Role {
            name: self.name.clone(),
            description: self.description.clone(),
            grants: self.grants.clone(),
            accounts: self.holders.iter().map(Holder::reference).collect(),
            tags: self.tags.clone(),
            created: self.created,
            creator: self.creator.clone(),
        }
    }
}
