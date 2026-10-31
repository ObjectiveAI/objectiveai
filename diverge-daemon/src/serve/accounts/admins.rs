//! The administrators the configuration names, listed as accounts.
//!
//! The rules:
//!
//! 1. Every identity the daemon's `admins` names is listed as an
//!    account with `admin` true, no description, roles or tags, and no
//!    `created` or `creator`: the configuration made it, not a request.
//! 2. An administrator that also has an account of its own is listed
//!    once, as that account, with `admin` true.
//! 3. `connected` is whether a client is connected as it now,
//!    administrators counted as accounts are.
//! 4. A grant's `within` and a request's filter read an administrator by
//!    the facts it has — its identity, whether it is connected — and
//!    never let it through a filter on roles, tags, creators or times
//!    made, which it does not have.
//! 5. The configuration is read once: an administrator's entry changes
//!    only as its `connected` does.
//! 6. With no account of its own, there is nothing to edit, tag or
//!    delete: those requests find no account. Its command log is the
//!    `admin` subject's.

use diverge_sdk::daemon::endpoints::accounts::list::client::request::Filter;
use diverge_sdk::daemon::endpoints::accounts::list::server::response::Account;
use diverge_sdk::daemon::grant::accounts::Over;
use diverge_sdk::shared::Identity;

use crate::daemon::Daemon;
use crate::judge::{self, Standing, filter};
use crate::store::accounts;

/// Rule 1: an administrator's entry.
pub fn entry(identity: &Identity, connected: bool) -> Account {
    Account {
        identity: identity.clone(),
        description: None,
        roles: Vec::new(),
        connected,
        tags: Vec::new(),
        created: None,
        creator: None,
        admin: true,
    }
}

/// Rules 1 to 4: the administrators `standing` may do `action` to and
/// `filter`, when given, lets through, but for those `rows` already
/// hold (rule 2), each as its entry.
pub async fn entries(daemon: &Daemon, standing: &Standing, action: Over, filter: Option<&Filter>, rows: &[accounts::Account]) -> Vec<Account> {
    let connected = daemon.live.admins_connected().await;
    daemon
        .authenticator
        .admins()
        .filter(|identity| !rows.iter().any(|row| &row.identity == *identity))
        .map(|identity| (identity, connected.contains(identity)))
        .filter(|(identity, connected)| judge::accounts::over_admin(standing, action, identity, *connected))
        .filter(|(identity, connected)| filter.is_none_or(|filter| filter::accounts::test_admin(filter, identity, *connected)))
        .map(|(identity, connected)| entry(identity, connected))
        .collect()
}
