//! Judging requests over outgoing providers.
//!
//! The shape of [`accounts`](super::accounts) with no tagging: a
//! making action is held or it is not, and an action over a provider
//! is allowed by a grant that holds it and whose `within` reaches the
//! provider — `any`, or the outgoing list filter as
//! [`filter::providers_outgoing::test`] reads it, with whether the
//! daemon holds a connection to the provider now, which the caller
//! knows. `ListFor` is judged here like any other action over the
//! provider it asks.

use diverge_sdk::daemon::endpoints::providers::outgoing::list::client::request::Filter;
use diverge_sdk::daemon::grant::Within;
use diverge_sdk::daemon::grant::providers_outgoing::{Make, Over, Permission};

use super::Standing;
use super::filter;
use crate::store::providers_outgoing::Outgoing;

/// Whether the standing may add an outgoing provider.
pub fn create(standing: &Standing) -> bool {
    standing
        .providers_outgoing()
        .any(|permission| matches!(permission, Permission::Make(makes) if makes.contains(&Make::Add)))
}

/// Whether the standing holds `action` over any outgoing provider at
/// all.
pub fn holds(standing: &Standing, action: Over) -> bool {
    standing
        .providers_outgoing()
        .any(|permission| matches!(permission, Permission::Over { actions, .. } if actions.contains(&action)))
}

/// Whether the standing may do `action` to `provider`, given whether
/// the daemon holds a connection to it now.
pub fn over(standing: &Standing, action: Over, provider: &Outgoing, connected: bool) -> bool {
    standing.providers_outgoing().any(|permission| match permission {
        Permission::Over { actions, within } => actions.contains(&action) && reaches(within, provider, connected),
        Permission::Make(_) => false,
    })
}

/// Whether a grant's `within` reaches the provider.
fn reaches(within: &Within<Filter>, provider: &Outgoing, connected: bool) -> bool {
    match within {
        Within::Any => true,
        Within::Only(filter) => filter::providers_outgoing::test(filter, provider, connected),
    }
}
