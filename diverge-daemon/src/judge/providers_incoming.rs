//! Judging requests over incoming credentials.
//!
//! The twin of [`providers_outgoing`](super::providers_outgoing),
//! over the credentials of providers that dial in; `connected` is
//! whether a provider is connected through the credential now.

use diverge_sdk::daemon::endpoints::providers::incoming::list::client::request::Filter;
use diverge_sdk::daemon::grant::Within;
use diverge_sdk::daemon::grant::providers_incoming::{Make, Over, Permission};

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
        Permission::Make(_) => false,
    })
}

/// Whether a grant's `within` reaches the credential.
fn reaches(within: &Within<Filter>, credential: &Incoming, connected: bool) -> bool {
    match within {
        Within::Any => true,
        Within::Only(filter) => filter::providers_incoming::test(filter, credential, connected),
    }
}
