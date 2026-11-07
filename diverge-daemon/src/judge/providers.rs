//! Judging requests over, and through, providers' records.
//!
//! The shape of [`daemons`](super::daemons): a making action — adding a
//! record — is held or it is not, judged by nothing else; an action over
//! a record — reading, listing, discovering, listening or messaging
//! through it, changing, deleting, enabling or disabling it — is allowed
//! by a grant that holds it and whose `within` reaches the record —
//! `any`, or the shared [`Tags`] test over the record's tags, since the
//! providers list filter names no tags; a tagging action by a grant that
//! holds it, reaches the record, and covers every tag named. The rules:
//!
//! 1. **Admin.** An administrator's standing is allowed everything, and
//!    reaches every record.
//! 2. **Make.** `add` is allowed by a grant holding it, and nothing else.
//! 3. **Over.** An action over a record is allowed by one grant holding
//!    it whose `within` reaches the record.
//! 4. **Tagging.** `tag` or `untag` is allowed by one grant holding it,
//!    reaching the record, and covering every tag named.
//! 5. **Reached.** What a standing reaches with an action over records
//!    is the identities of the records rule 3 allows it over, read from
//!    records handed in — the judge loads nothing.
//! 6. **Tags.** A `within` of tags reaches a record carrying every one
//!    of its `all_tags` and, where `any_tags` is given, any one of them.

use std::collections::HashSet;

use diverge_sdk::daemon::grant::providers::{Make, Over, Permission};
use diverge_sdk::daemon::grant::{Tagging, Within};
use diverge_sdk::shared::Identity;
use diverge_sdk::shared::permission::Tags;

use super::Standing;
use crate::store::providers::Record;

/// Whether the standing may do the making `action` — rules 1 and 2.
pub fn make(standing: &Standing, action: Make) -> bool {
    if standing.admin() {
        return true;
    }
    standing
        .providers()
        .any(|permission| matches!(permission, Permission::Make(makes) if makes.contains(&action)))
}

/// Whether the standing holds `action` over any record at all.
pub fn holds(standing: &Standing, action: Over) -> bool {
    if standing.admin() {
        return true;
    }
    standing
        .providers()
        .any(|permission| matches!(permission, Permission::Over { actions, .. } if actions.contains(&action)))
}

/// Whether the standing may do `action` to the record — rules 1 and 3.
pub fn over(standing: &Standing, action: Over, record: &Record) -> bool {
    if standing.admin() {
        return true;
    }
    standing.providers().any(|permission| match permission {
        Permission::Over { actions, within } => actions.contains(&action) && reaches(within, &record.tags),
        _ => false,
    })
}

/// The identities of every one of `records` the standing may do
/// `action` to — rule 5: every one of them for an administrator, none
/// without a grant of `action`.
pub fn reached(standing: &Standing, action: Over, records: &[Record]) -> HashSet<Identity> {
    records.iter().filter(|record| over(standing, action, record)).map(|record| record.identity.clone()).collect()
}

/// Whether the standing holds the tagging `action` over any record at
/// all.
pub fn holds_tagging(standing: &Standing, action: Tagging) -> bool {
    if standing.admin() {
        return true;
    }
    standing
        .providers()
        .any(|permission| matches!(permission, Permission::Tags { actions, .. } if actions.contains(&action)))
}

/// Whether the standing may put `tags` on the record, or take them
/// off, as `action` says — rules 1 and 4: one grant must hold the
/// action, reach the record, and name every one of the tags or `any`.
pub fn tagging(standing: &Standing, action: Tagging, record: &Record, tags: &[String]) -> bool {
    if standing.admin() {
        return true;
    }
    standing.providers().any(|permission| match permission {
        Permission::Tags {
            actions,
            within,
            tags: scope,
        } => actions.contains(&action) && reaches(within, &record.tags) && covers(scope, tags),
        _ => false,
    })
}

/// Whether a grant's `within` reaches a record carrying `tags` — rule 6.
fn reaches(within: &Within<Tags>, tags: &[String]) -> bool {
    match within {
        Within::Any => true,
        Within::Only(test) => {
            test.all_tags.iter().all(|tag| tags.contains(tag))
                && (test.any_tags.is_empty() || test.any_tags.iter().any(|tag| tags.contains(tag)))
        }
    }
}

/// Whether a grant's tag scope names every one of `tags`.
fn covers(scope: &Within<Vec<String>>, tags: &[String]) -> bool {
    match scope {
        Within::Any => true,
        Within::Only(allowed) => tags.iter().all(|tag| allowed.contains(tag)),
    }
}
