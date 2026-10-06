//! Grants over volumes.
//!
//! What a role may allow over volumes, in the two shapes
//! [`grant`](crate::daemon::grant) states: [`Make`], the action that
//! brings one into being; [`Over`], the actions over those that exist;
//! and [`Permission`], one grant's worth of either. A volume carries no
//! tags, so there is no tagging shape.

mod make;
mod over;
mod permission;

pub use make::*;
pub use over::*;
pub use permission::*;
