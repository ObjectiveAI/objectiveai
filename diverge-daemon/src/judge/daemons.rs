//! Who may do what through the daemons this daemon reaches.

use diverge_sdk::daemon::grant::daemons::{Action, Permission};

use super::Standing;

/// Whether any grant of `standing`'s allows `action` through the remote
/// daemons: the grant is a list of acts over all of them, since the
/// remote daemons are the configuration's and none is any account's.
pub fn holds(standing: &Standing, action: Action) -> bool {
    if standing.admin() {
        return true;
    }
    standing.daemons().any(|Permission(actions)| actions.contains(&action))
}
