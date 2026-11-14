//! Grants through providers, over the daemons connected to them.
//!
//! What a role may allow through the daemon's providers: [`Over`],
//! defined once in [`shared::permission`](crate::shared::permission)
//! and re-exported; and [`Permission`], one grant's worth, reaching
//! the providers a [`Reach`](crate::daemon::grant::providers::Reach)
//! lets through — by identity, by their records' tags, or both.

mod permission;

pub use crate::shared::permission::providers_daemons::Over;
pub use permission::*;
