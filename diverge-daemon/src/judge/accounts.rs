//! Judging requests over accounts.
//!
//! Every function answers whether SOME one grant of the standing
//! allows what is asked, as the wire's rule has it: a making action
//! is held or it is not; an action over an account is allowed by a
//! grant that holds the action and whose `within` reaches the account;
//! a tagging action by a grant that holds it, reaches the account, and
//! covers every tag named. `within` reaches an account when it is
//! `any`, or when the account passes the grant's filter as
//! [`filter::accounts::test`] reads
//! it — with whether a client is connected as the account now, which
//! the filter may ask about and which is the caller's to know.

use diverge_sdk::daemon::grant::accounts::{Make, Over, Permission};
use diverge_sdk::daemon::grant::{Tagging, Within};

use super::filter;
use super::Standing;
use crate::store::accounts::Account;

/// Whether the standing may create an account.
pub fn create(standing: &Standing) -> bool {
    standing
        .accounts()
        .any(|permission| matches!(permission, Permission::Make(makes) if makes.contains(&Make::Create)))
}

/// Whether the standing holds `action` over any account at all —
/// what decides `Forbidden` before anything is loaded, so that a
/// caller with no such grant learns nothing about what exists.
pub fn holds(standing: &Standing, action: Over) -> bool {
    standing
        .accounts()
        .any(|permission| matches!(permission, Permission::Over { actions, .. } if actions.contains(&action)))
}

/// Whether the standing may do `action` to `account`, given whether a
/// client is connected as it now.
pub fn over(standing: &Standing, action: Over, account: &Account, connected: bool) -> bool {
    standing.accounts().any(|permission| match permission {
        Permission::Over { actions, within } => actions.contains(&action) && reaches(within, account, connected),
        _ => false,
    })
}

/// Whether the standing holds the tagging `action` over any account
/// at all.
pub fn holds_tagging(standing: &Standing, action: Tagging) -> bool {
    standing
        .accounts()
        .any(|permission| matches!(permission, Permission::Tags { actions, .. } if actions.contains(&action)))
}

/// Whether the standing may put `tags` on `account`, or take them off,
/// as `action` says: one grant must hold the action, reach the
/// account, and name every one of the tags or `any`.
pub fn tagging(standing: &Standing, action: Tagging, account: &Account, connected: bool, tags: &[String]) -> bool {
    standing.accounts().any(|permission| match permission {
        Permission::Tags {
            actions,
            within,
            tags: scope,
        } => actions.contains(&action) && reaches(within, account, connected) && covers(scope, tags),
        _ => false,
    })
}

/// Whether a grant's `within` reaches the account.
fn reaches(within: &Within<diverge_sdk::daemon::endpoints::accounts::list::client::request::Filter>, account: &Account, connected: bool) -> bool {
    match within {
        Within::Any => true,
        Within::Only(filter) => filter::accounts::test(filter, account, connected),
    }
}

/// Whether a grant's tag scope names every one of `tags`.
fn covers(scope: &Within<Vec<String>>, tags: &[String]) -> bool {
    match scope {
        Within::Any => true,
        Within::Only(allowed) => tags.iter().all(|tag| allowed.contains(tag)),
    }
}
