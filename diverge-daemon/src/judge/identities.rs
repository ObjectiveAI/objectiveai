//! Who may do what over the host's identities.

use diverge_sdk::daemon::grant::identities::{Action, Permission};

use super::Standing;

/// Whether any grant of `standing`'s allows `action` over the host's
/// identities: the grant is a list of acts over all of them, since the
/// identities are the host's files and none is any account's.
pub fn holds(standing: &Standing, action: Action) -> bool {
    if standing.admin() {
        return true;
    }
    standing.identities().any(|Permission(actions)| actions.contains(&action))
}
