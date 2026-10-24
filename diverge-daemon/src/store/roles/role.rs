//! One role, as the store holds it.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::roles::list::server::response;
use diverge_sdk::daemon::grant::Grant;
use diverge_sdk::shared::Identity;

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

/// An account holding a role: its row and the identity it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Holder {
    /// The account.
    pub id: AccountId,
    /// The identity the account is: how the wire names it.
    pub identity: Identity,
}

impl Role {
    /// The role as a list reports it.
    pub fn report(&self) -> response::Role {
        response::Role {
            name: self.name.clone(),
            description: self.description.clone(),
            grants: self.grants.clone(),
            accounts: self.holders.iter().map(|holder| holder.identity.clone()).collect(),
            tags: self.tags.clone(),
            created: self.created,
            creator: self.creator.clone(),
        }
    }
}
