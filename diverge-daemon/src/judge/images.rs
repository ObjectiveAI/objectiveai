//! Judging requests over images.
//!
//! The shape of [`events`](super::events): an image is named by its
//! digest and is no record of the daemon's, so a grant over images
//! names nothing and reaches no narrower than every image. An action is
//! held or it is not.

use diverge_sdk::daemon::grant::images::{Action, Permission};

use super::Standing;

/// Whether the standing holds `action` over images.
pub fn holds(standing: &Standing, action: Action) -> bool {
    if standing.admin() {
        return true;
    }
    standing
        .images()
        .any(|Permission(actions)| actions.contains(&action))
}
