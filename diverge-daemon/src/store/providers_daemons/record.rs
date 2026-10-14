//! One daemon record, as the store holds it.

use chrono::{DateTime, Utc};
use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::providers::daemons::list::server::response;
use diverge_sdk::daemon::endpoints::providers::daemons::{self, Link};
use diverge_sdk::daemon::endpoints::providers::outgoing::{Kind, Mode};

use crate::store::DaemonId;

/// A live daemon record row, with its accounts: the name, the links,
/// the accounts whole — each mode with its credential, which the
/// wire never answers — its tags, and who added it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// The row.
    pub id: DaemonId,
    /// The name, as the add gave it.
    pub name: String,
    /// The links, as the add or the last edit gave them.
    pub links: Vec<Link>,
    /// The accounts, in the order they were added.
    pub accounts: Vec<Account>,
    /// Its tags, sorted bytewise.
    pub tags: Vec<String>,
    /// When the add made it.
    pub created: DateTime<Utc>,
    /// Who added it.
    pub creator: Creator,
}

/// One account on a daemon record: its name and its mode, credential
/// and all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    /// The name, as its add gave it.
    pub name: String,
    /// How this daemon authenticates to the remote as it.
    pub mode: Mode,
    /// When its add made it.
    pub created: DateTime<Utc>,
}

impl Account {
    /// The mode's kind, without the credential.
    pub fn kind(&self) -> Kind {
        match self.mode {
            Mode::Unbrokered { .. } => Kind::Unbrokered,
        }
    }

    /// The account as a list reports it: its name and its kind.
    pub fn report(&self) -> daemons::Account {
        daemons::Account {
            name: self.name.clone(),
            kind: self.kind(),
        }
    }
}

impl Record {
    /// The account of the name, if the record has one.
    pub fn account(&self, name: &str) -> Option<&Account> {
        self.accounts.iter().find(|account| account.name == name)
    }

    /// The record as a list reports it, given the names of the
    /// accounts this daemon holds a connection as now.
    pub fn report(&self, connected: Vec<String>) -> response::Daemon {
        response::Daemon {
            name: self.name.clone(),
            links: self.links.clone(),
            accounts: self.accounts.iter().map(Account::report).collect(),
            connected,
            created: self.created,
            creator: self.creator.clone(),
            tags: self.tags.clone(),
        }
    }
}
