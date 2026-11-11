//! Grants over remote daemons.
//!
//! What a role may allow over remote daemons' records, in the three
//! shapes [`grant`](crate::daemon::grant) states: [`Make`] and [`Over`],
//! defined once in [`shared::permission`](crate::shared::permission) and
//! re-exported; and [`Permission`], one grant's worth of either, or of
//! tagging. A relay is judged as [`Over::Relay`] over the record its
//! first ID is, and nothing else.

mod permission;

pub use crate::shared::permission::daemons::{Make, Over};
pub use permission::*;
