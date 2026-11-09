//! Judging requests over, and through, remote daemons' records.
//!
//! The shape of [`volumes`](super::volumes): a making action — adding a
//! record — is held or it is not, judged by nothing else; an action over
//! a record — reading, changing, deleting, holding a connected tool on
//! it, relaying to it — is allowed by a grant that holds it and whose
//! `within` reaches the record — `any`, or the daemons list filter as
//! [`filter::daemons::test`] reads it, over the record as a list reports
//! it, its tags and its connection included; a tagging action by a grant
//! that holds it, reaches the record, and covers every tag named. A
//! relay is judged as `relay` over the record its first ID is.

use std::collections::HashSet;

use diverge_sdk::daemon::endpoints::daemons::list::client::request::Filter;
use diverge_sdk::daemon::endpoints::daemons::{Daemon, DaemonId};
use diverge_sdk::daemon::grant::daemons::{Make, Over, Permission};
use diverge_sdk::daemon::grant::{Tagging, Within};

use super::Standing;
use super::filter;
use crate::store::daemons::Record;

/// Whether the standing may do the making `action`.
pub fn make(standing: &Standing, action: Make) -> bool {
    if standing.admin() {
        return true;
    }
    standing
        .daemons()
        .any(|permission| matches!(permission, Permission::Make(makes) if makes.contains(&action)))
}

/// Whether the standing holds `action` over any record at all.
pub fn holds(standing: &Standing, action: Over) -> bool {
    if standing.admin() {
        return true;
    }
    standing
        .daemons()
        .any(|permission| matches!(permission, Permission::Over { actions, .. } if actions.contains(&action)))
}

/// Whether the standing may do `action` to the record.
pub fn over(standing: &Standing, action: Over, daemon: &Daemon) -> bool {
    if standing.admin() {
        return true;
    }
    standing.daemons().any(|permission| match permission {
        Permission::Over { actions, within } => actions.contains(&action) && reaches(within, daemon),
        _ => false,
    })
}

/// Every one of `records` the standing may do `action` to, each judged
/// as a list reports it, connected where its ID is in `connected`:
/// every one of them for an administrator, none without a grant of
/// `action`.
pub fn reached(standing: &Standing, action: Over, records: Vec<Record>, connected: &HashSet<DaemonId>) -> Vec<Record> {
    records
        .into_iter()
        .filter(|record| over(standing, action, &record.report(connected.contains(&record.id))))
        .collect()
}

/// Whether the standing holds the tagging `action` over any record at
/// all.
pub fn holds_tagging(standing: &Standing, action: Tagging) -> bool {
    if standing.admin() {
        return true;
    }
    standing
        .daemons()
        .any(|permission| matches!(permission, Permission::Tags { actions, .. } if actions.contains(&action)))
}

/// Whether the standing may put `tags` on the record, or take them
/// off, as `action` says: one grant must hold the action, reach the
/// identity, and name every one of the tags or `any`.
pub fn tagging(standing: &Standing, action: Tagging, daemon: &Daemon, tags: &[String]) -> bool {
    if standing.admin() {
        return true;
    }
    standing.daemons().any(|permission| match permission {
        Permission::Tags {
            actions,
            within,
            tags: scope,
        } => actions.contains(&action) && reaches(within, daemon) && covers(scope, tags),
        _ => false,
    })
}

/// Whether a grant's `within` reaches the record.
fn reaches(within: &Within<Filter>, daemon: &Daemon) -> bool {
    match within {
        Within::Any => true,
        Within::Only(filter) => filter::daemons::test(filter, daemon),
    }
}

/// Whether a grant's tag scope names every one of `tags`.
fn covers(scope: &Within<Vec<String>>, tags: &[String]) -> bool {
    match scope {
        Within::Any => true,
        Within::Only(allowed) => tags.iter().all(|tag| allowed.contains(tag)),
    }
}
