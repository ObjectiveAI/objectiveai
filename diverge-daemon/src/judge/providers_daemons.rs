//! Judging requests over daemon records.
//!
//! The shape of [`accounts`](super::accounts): a making action is held
//! or it is not; an action over a daemon record is allowed by a grant
//! that holds it and whose `within` reaches the record — `any`, or the
//! daemons list filter as [`filter::providers_daemons::test`] reads it,
//! with whether this daemon holds a connection to the daemon now, which
//! the caller knows; a tagging action by a grant that holds it, reaches
//! the record, and covers every tag named.

use diverge_sdk::daemon::endpoints::providers::daemons::list::client::request::Filter;
use diverge_sdk::daemon::grant::providers_daemons::{Make, Over, Permission};
use diverge_sdk::daemon::grant::{Tagging, Within};

use super::Standing;
use super::filter;
use crate::store::providers_daemons::Record;

/// Whether the standing may add a daemon record.
pub fn create(standing: &Standing) -> bool {
    standing
        .providers_daemons()
        .any(|permission| matches!(permission, Permission::Make(makes) if makes.contains(&Make::Add)))
}

/// Whether the standing holds `action` over any daemon record at all.
pub fn holds(standing: &Standing, action: Over) -> bool {
    standing
        .providers_daemons()
        .any(|permission| matches!(permission, Permission::Over { actions, .. } if actions.contains(&action)))
}

/// Whether the standing may do `action` to `record`, given whether this
/// daemon holds a connection to it now.
pub fn over(standing: &Standing, action: Over, record: &Record, connected: bool) -> bool {
    standing.providers_daemons().any(|permission| match permission {
        Permission::Over { actions, within } => actions.contains(&action) && reaches(within, record, connected),
        _ => false,
    })
}

/// Whether the standing holds the tagging `action` over any daemon
/// record at all.
pub fn holds_tagging(standing: &Standing, action: Tagging) -> bool {
    standing
        .providers_daemons()
        .any(|permission| matches!(permission, Permission::Tags { actions, .. } if actions.contains(&action)))
}

/// Whether the standing may put `tags` on the record, or take them off,
/// as `action` says: one grant must hold the action, reach the
/// provider, and name every one of the tags or `any`.
pub fn tagging(standing: &Standing, action: Tagging, record: &Record, connected: bool, tags: &[String]) -> bool {
    standing.providers_daemons().any(|permission| match permission {
        Permission::Tags {
            actions,
            within,
            tags: scope,
        } => actions.contains(&action) && reaches(within, record, connected) && covers(scope, tags),
        _ => false,
    })
}

/// Whether a grant's `within` reaches the record.
fn reaches(within: &Within<Filter>, record: &Record, connected: bool) -> bool {
    match within {
        Within::Any => true,
        Within::Only(filter) => filter::providers_daemons::test(filter, record, connected),
    }
}

/// Whether a grant's tag scope names every one of `tags`.
fn covers(scope: &Within<Vec<String>>, tags: &[String]) -> bool {
    match scope {
        Within::Any => true,
        Within::Only(allowed) => tags.iter().all(|tag| allowed.contains(tag)),
    }
}
