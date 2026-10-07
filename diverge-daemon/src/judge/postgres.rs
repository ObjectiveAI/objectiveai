//! Judging requests over the database.
//!
//! One shape: the daemon serves exactly one database, so a grant over
//! it names nothing and reaches no narrower than the whole. An action
//! is held or it is not.

use diverge_sdk::daemon::grant::postgres::{Action, Permission};

use super::Standing;

/// Whether the standing holds `action` over the database.
pub fn holds(standing: &Standing, action: Action) -> bool {
    standing
        .postgres()
        .any(|Permission(actions)| actions.contains(&action))
}
