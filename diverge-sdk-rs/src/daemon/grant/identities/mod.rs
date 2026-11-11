//! Grants over the host's identities.
//!
//! What a role may allow over the identities signed in on the daemon's
//! host, in the three shapes [`grant`](crate::daemon::grant) states:
//! [`Make`], the action that brings one onto the host, and [`Over`], the
//! actions over those there — both defined once in
//! [`shared::permission`](crate::shared::permission) and re-exported;
//! and [`Permission`], one grant's worth of either, or of tagging.

mod permission;

pub use crate::shared::permission::identities::{Make, Over};
pub use permission::*;
