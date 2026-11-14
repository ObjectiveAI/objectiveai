//! Grants over providers.
//!
//! What a role may allow over providers' records, in the three shapes
//! [`grant`](crate::daemon::grant) states: [`Make`] and [`Over`],
//! defined once in [`shared::permission`](crate::shared::permission)
//! and re-exported; and [`Permission`], one grant's worth of either,
//! or of tagging.
//!
//! The providers list filter names no tags, so a grant over providers
//! reaches by a test of its own, [`Reach`]: the providers named by
//! identity, those whose records carry some tags, or both. The same
//! test is the `within` of the two grants that act THROUGH a provider:
//! [`providers_daemons`](crate::daemon::grant::providers_daemons) and
//! [`images`](crate::daemon::grant::images).

mod permission;
mod reach;

pub use crate::shared::permission::providers::{Make, Over};
pub use permission::*;
pub use reach::*;
