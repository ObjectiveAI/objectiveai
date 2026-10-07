//! Judging requests over tool templates.
//!
//! The shape of [`accounts`](super::accounts): a making action is held
//! or it is not; an action over a template is allowed by a grant that
//! holds it and whose `within` reaches the template — `any`, or the
//! family's list filter as [`filter::tools_templates::test`] reads it, with
//! whether some tool was made from the template, which the caller
//! knows; a tagging action by a grant that holds it, reaches the
//! template, and covers every tag named.

use diverge_sdk::daemon::endpoints::tools::templates::list::client::request::Filter;
use diverge_sdk::daemon::grant::tools_templates::{Make, Over, Permission};
use diverge_sdk::daemon::grant::{Tagging, Within};

use super::Standing;
use super::filter;
use crate::store::tools_templates::Record;

/// Whether the standing may create a tool template.
pub fn create(standing: &Standing) -> bool {
    standing
        .tools_templates()
        .any(|permission| matches!(permission, Permission::Make(makes) if makes.contains(&Make::Create)))
}

/// Whether the standing holds `action` over any tool template at all.
pub fn holds(standing: &Standing, action: Over) -> bool {
    standing
        .tools_templates()
        .any(|permission| matches!(permission, Permission::Over { actions, .. } if actions.contains(&action)))
}

/// Whether the standing may do `action` to `record`, given whether
/// some tool was made from it.
pub fn over(standing: &Standing, action: Over, record: &Record, in_use: bool) -> bool {
    standing.tools_templates().any(|permission| match permission {
        Permission::Over { actions, within } => actions.contains(&action) && reaches(within, record, in_use),
        _ => false,
    })
}

/// Whether the standing holds the tagging `action` over any tool
/// template at all.
pub fn holds_tagging(standing: &Standing, action: Tagging) -> bool {
    standing
        .tools_templates()
        .any(|permission| matches!(permission, Permission::Tags { actions, .. } if actions.contains(&action)))
}

/// Whether the standing may put `tags` on `record`, or take them off,
/// as `action` says.
pub fn tagging(standing: &Standing, action: Tagging, record: &Record, in_use: bool, tags: &[String]) -> bool {
    standing.tools_templates().any(|permission| match permission {
        Permission::Tags {
            actions,
            within,
            tags: scope,
        } => actions.contains(&action) && reaches(within, record, in_use) && covers(scope, tags),
        _ => false,
    })
}

/// Whether a grant's `within` reaches the template.
fn reaches(within: &Within<Filter>, record: &Record, in_use: bool) -> bool {
    match within {
        Within::Any => true,
        Within::Only(filter) => filter::tools_templates::test(filter, record, in_use),
    }
}

/// Whether a grant's tag scope names every one of `tags`.
fn covers(scope: &Within<Vec<String>>, tags: &[String]) -> bool {
    match scope {
        Within::Any => true,
        Within::Only(allowed) => tags.iter().all(|tag| allowed.contains(tag)),
    }
}
