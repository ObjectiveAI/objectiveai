//! Grants over the vault's keys.
//!
//! What a role may allow over the vault, in the three shapes
//! [`grant`](crate::daemon::grant) states: [`Make`], the one action
//! that brings a key into being, and [`Over`], the actions over keys
//! that exist — both defined once in
//! [`shared::permission`](crate::shared::permission) and re-exported;
//! and [`Permission`], one grant's worth of either, or of tagging.

mod permission;

pub use crate::shared::permission::vault::{Make, Over};
pub use permission::*;
