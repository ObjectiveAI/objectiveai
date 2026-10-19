//! Grants over outgoing providers.
//!
//! What a role may allow over outgoing providers, in the three shapes
//! [`grant`](crate::daemon::grant) states: [`Make`], the actions that
//! bring one into being, and [`Over`], the actions over those that
//! exist — both defined once in
//! [`shared::permission`](crate::shared::permission) and re-exported;
//! and [`Permission`], one grant's worth of either, or of tagging.

mod permission;

pub use crate::shared::permission::providers_outgoing::{Make, Over};
pub use permission::*;
