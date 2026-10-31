//! One account, as the store holds it.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::accounts::list::server::response;
use diverge_sdk::shared::Identity;

use crate::store::AccountId;

/// One account, as the store holds it: the identity it is, what it is
/// for, the roles it holds, its tags, and when and by whom it was made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    /// The row's id.
    pub id: AccountId,
    /// The identity the account is.
    pub identity: Identity,
    /// What it is for.
    pub description: Option<String>,
    /// The roles it holds, by name, sorted.
    pub roles: Vec<String>,
    /// Its tags, sorted.
    pub tags: Vec<String>,
    /// When it was made.
    pub created: DateTime<Utc>,
    /// Who made it.
    pub creator: Creator,
}

impl Account {
    /// The account as a list reports it, `connected` or not.
    pub fn report(&self, connected: bool) -> response::Account {
        response::Account {
            identity: self.identity.clone(),
            description: self.description.clone(),
            roles: self.roles.clone(),
            connected,
            tags: self.tags.clone(),
            created: Some(self.created),
            creator: Some(self.creator.clone()),
            admin: false,
        }
    }
}
