//! Grants over connected tools.
//!
//! What a role may allow over connected tools — other daemons'
//! standalone tools, held here — in the three shapes
//! [`grant`](crate::daemon::grant) states: [`Make`], the action that
//! brings one into being, and [`Over`], the actions over those that
//! exist — both defined once in
//! [`shared::permission`](crate::shared::permission) and re-exported;
//! and [`Permission`], one grant's worth of either, or of tagging.

mod permission;

pub use crate::shared::permission::connected_standalone_tools::{Make, Over};
pub use permission::*;
