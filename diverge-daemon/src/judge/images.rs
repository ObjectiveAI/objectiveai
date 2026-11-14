//! Judging requests over images: the daemon's Stores, provider by
//! provider.
//!
//! The shape of [`providers_daemons`](super::providers_daemons): an
//! action — building into a Store, pushing into one, listing one — is
//! allowed on a provider by a grant that holds it and whose `within`
//! reaches the provider's record — `any`, or a
//! [`Reach`](diverge_sdk::daemon::grant::providers::Reach) over the
//! record's identity and its tags. The rules:
//!
//! 1. **Admin.** An administrator's standing is allowed everything, and
//!    reaches every provider.
//! 2. **Over.** An action on a provider's Store is allowed by one
//!    grant holding it whose `within` reaches the provider's record, as
//!    the providers judge reads a `within`.
//! 3. **The provider alone.** Nothing of an image is judged: not its
//!    digest, not when it came to be owned. A Store is the daemon's,
//!    and a grant reaches it whole or not at all.
//! 4. **Reached.** What a standing reaches with an action is the
//!    identities of the records rule 2 allows it on, read from records
//!    handed in — the judge loads nothing.

use std::collections::HashSet;

use diverge_sdk::daemon::grant::images::{Action, Permission};
use diverge_sdk::shared::Identity;

use super::Standing;
use crate::store::providers::Record;

/// Whether the standing holds `action` on any provider at all —
/// rule 1.
pub fn holds(standing: &Standing, action: Action) -> bool {
    if standing.admin() {
        return true;
    }
    standing.images().any(|Permission::Over { actions, .. }| actions.contains(&action))
}

/// Whether the standing may do `action` on the Store of the record's
/// provider — rules 1 to 3.
pub fn over(standing: &Standing, action: Action, record: &Record) -> bool {
    if standing.admin() {
        return true;
    }
    standing
        .images()
        .any(|Permission::Over { actions, within }| actions.contains(&action) && super::providers::reaches(within, record))
}

/// The identities of every one of `records` the standing may do
/// `action` on — rule 4: every one of them for an administrator, none
/// without a grant of `action`.
pub fn reached(standing: &Standing, action: Action, records: &[Record]) -> HashSet<Identity> {
    records.iter().filter(|record| over(standing, action, record)).map(|record| record.identity.clone()).collect()
}
