//! Who may do what over the daemon's providers.

use diverge_sdk::daemon::grant::providers::{Action, Permission};

use super::Standing;

/// Whether any grant of `standing`'s allows `action` over the
/// providers: the grant is a list of acts over all of them, since the
/// providers are the configuration's and none is any account's.
pub fn holds(standing: &Standing, action: Action) -> bool {
    if standing.admin() {
        return true;
    }
    standing.providers().any(|Permission(actions)| actions.contains(&action))
}
