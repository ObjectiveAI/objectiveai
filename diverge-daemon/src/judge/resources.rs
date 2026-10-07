//! Judging requests over resources.
//!
//! The shape of [`accounts`](super::accounts): a making action — the
//! upload — is held or it is not; an action over a resource is
//! allowed by a grant that holds it and whose `within` reaches the
//! resource — `any`, or the resources list filter as
//! [`filter::resources::test`] reads it, with whether some container
//! mounts the resource, which the caller knows; a tagging action by a
//! grant that holds it, reaches the resource, and covers every tag
//! named. Where a transfer lands is judged by its own grant, not here.

use diverge_sdk::daemon::endpoints::resources::list::client::request::Filter;
use diverge_sdk::daemon::grant::resources::{Make, Over, Permission};
use diverge_sdk::daemon::grant::{Tagging, Within};

use super::Standing;
use super::filter;
use crate::store::resources::Record;

/// Whether the standing may upload a resource.
pub fn create(standing: &Standing) -> bool {
    standing
        .resources()
        .any(|permission| matches!(permission, Permission::Make(makes) if makes.contains(&Make::Upload)))
}

/// Whether the standing holds `action` over any resource at all.
pub fn holds(standing: &Standing, action: Over) -> bool {
    standing
        .resources()
        .any(|permission| matches!(permission, Permission::Over { actions, .. } if actions.contains(&action)))
}

/// Whether the standing may do `action` to `record`, given whether
/// some container mounts it.
pub fn over(standing: &Standing, action: Over, record: &Record, in_use: bool) -> bool {
    standing.resources().any(|permission| match permission {
        Permission::Over { actions, within } => actions.contains(&action) && reaches(within, record, in_use),
        _ => false,
    })
}

/// Whether the standing holds the tagging `action` over any resource
/// at all.
pub fn holds_tagging(standing: &Standing, action: Tagging) -> bool {
    standing
        .resources()
        .any(|permission| matches!(permission, Permission::Tags { actions, .. } if actions.contains(&action)))
}

/// Whether the standing may put `tags` on `record`, or take them off,
/// as `action` says.
pub fn tagging(standing: &Standing, action: Tagging, record: &Record, in_use: bool, tags: &[String]) -> bool {
    standing.resources().any(|permission| match permission {
        Permission::Tags {
            actions,
            within,
            tags: scope,
        } => actions.contains(&action) && reaches(within, record, in_use) && covers(scope, tags),
        _ => false,
    })
}

/// Whether a grant's `within` reaches the resource.
fn reaches(within: &Within<Filter>, record: &Record, in_use: bool) -> bool {
    match within {
        Within::Any => true,
        Within::Only(filter) => filter::resources::test(filter, record, in_use),
    }
}

/// Whether a grant's tag scope names every one of `tags`.
fn covers(scope: &Within<Vec<String>>, tags: &[String]) -> bool {
    match scope {
        Within::Any => true,
        Within::Only(allowed) => tags.iter().all(|tag| allowed.contains(tag)),
    }
}
