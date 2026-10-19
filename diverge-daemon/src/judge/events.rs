//! Judging requests over the events.
//!
//! The shape of [`postgres`](super::postgres): the daemon keeps exactly
//! one graph of events, so a grant over it names nothing and reaches
//! no narrower than the whole. An action is held or it is not.

use diverge_sdk::daemon::grant::events::{Action, Permission};

use super::Standing;

/// Whether the standing holds `action` over the events.
pub fn holds(standing: &Standing, action: Action) -> bool {
    standing
        .events()
        .any(|Permission(actions)| actions.contains(&action))
}
