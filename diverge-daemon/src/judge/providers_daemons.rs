//! Judging requests through providers, over the daemons connected to
//! them.
//!
//! The shape of [`providers`](super::providers)'s actions over records:
//! an action — listing the announced identities, listening for messages,
//! sending one — is allowed through a provider by a grant that holds it
//! and whose `within` reaches the provider's record — `any`, or a
//! [`Reach`](diverge_sdk::daemon::grant::providers::Reach) over the
//! record's identity and its tags. The rules:
//!
//! 1. **Admin.** An administrator's standing is allowed everything, and
//!    reaches every provider.
//! 2. **Over.** An action through a provider is allowed by one grant
//!    holding it whose `within` reaches the provider's record, as the
//!    providers judge reads a `within`.
//! 3. **Reached.** What a standing reaches with an action is the
//!    identities of the records rule 2 allows it through, read from
//!    records handed in — the judge loads nothing.

use std::collections::HashSet;

use diverge_sdk::daemon::grant::providers_daemons::{Over, Permission};
use diverge_sdk::shared::Identity;

use super::Standing;
use crate::store::providers::Record;

/// Whether the standing holds `action` through any provider at all —
/// rule 1.
pub fn holds(standing: &Standing, action: Over) -> bool {
    if standing.admin() {
        return true;
    }
    standing.providers_daemons().any(|Permission::Over { actions, .. }| actions.contains(&action))
}

/// Whether the standing may do `action` through the provider of the
/// record — rules 1 and 2.
pub fn over(standing: &Standing, action: Over, record: &Record) -> bool {
    if standing.admin() {
        return true;
    }
    standing
        .providers_daemons()
        .any(|Permission::Over { actions, within }| actions.contains(&action) && super::providers::reaches(within, record))
}

/// The identities of every one of `records` the standing may do
/// `action` through — rule 3: every one of them for an administrator,
/// none without a grant of `action`.
pub fn reached(standing: &Standing, action: Over, records: &[Record]) -> HashSet<Identity> {
    records.iter().filter(|record| over(standing, action, record)).map(|record| record.identity.clone()).collect()
}
