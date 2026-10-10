//! Judging requests over incoming credentials.
//!
//! The twin of [`providers_outgoing`](super::providers_outgoing),
//! over the credentials of providers that dial in, tagging included;
//! `connected` is whether a provider is connected through the
//! credential now.

use diverge_sdk::daemon::endpoints::providers::incoming::list::client::request::Filter;
use diverge_sdk::daemon::grant::providers_incoming::{Make, Over, Permission};
use diverge_sdk::daemon::grant::{Tagging, Within};

use super::Standing;
use super::filter;
use crate::store::providers_incoming::Incoming;

/// Whether the standing may add an incoming credential.
pub fn create(standing: &Standing) -> bool {
    standing
        .providers_incoming()
        .any(|permission| matches!(permission, Permission::Make(makes) if makes.contains(&Make::Add)))
}

/// Whether the standing holds `action` over any incoming credential
/// at all.
pub fn holds(standing: &Standing, action: Over) -> bool {
    standing
        .providers_incoming()
        .any(|permission| matches!(permission, Permission::Over { actions, .. } if actions.contains(&action)))
}

/// Whether the standing may do `action` to `credential`, given whether
/// a provider is connected through it now.
pub fn over(standing: &Standing, action: Over, credential: &Incoming, connected: bool) -> bool {
    standing.providers_incoming().any(|permission| match permission {
        Permission::Over { actions, within } => actions.contains(&action) && reaches(within, credential, connected),
        _ => false,
    })
}

/// Whether the standing holds the tagging `action` over any incoming
/// credential at all.
pub fn holds_tagging(standing: &Standing, action: Tagging) -> bool {
    standing
        .providers_incoming()
        .any(|permission| matches!(permission, Permission::Tags { actions, .. } if actions.contains(&action)))
}

/// Whether the standing may put `tags` on the credential, or take
/// them off, as `action` says: one grant must hold the action, reach
/// the credential, and name every one of the tags or `any`.
pub fn tagging(standing: &Standing, action: Tagging, credential: &Incoming, connected: bool, tags: &[String]) -> bool {
    standing.providers_incoming().any(|permission| match permission {
        Permission::Tags {
            actions,
            within,
            tags: scope,
        } => actions.contains(&action) && reaches(within, credential, connected) && covers(scope, tags),
        _ => false,
    })
}

/// Whether a grant's `within` reaches the credential.
fn reaches(within: &Within<Filter>, credential: &Incoming, connected: bool) -> bool {
    match within {
        Within::Any => true,
        Within::Only(filter) => filter::providers_incoming::test(filter, credential, connected),
    }
}

/// Whether a grant's tag scope names every one of `tags`.
fn covers(scope: &Within<Vec<String>>, tags: &[String]) -> bool {
    match scope {
        Within::Any => true,
        Within::Only(allowed) => tags.iter().all(|tag| allowed.contains(tag)),
    }
}
