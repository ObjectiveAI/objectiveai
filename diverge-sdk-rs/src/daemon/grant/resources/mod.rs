//! Grants over resources.
//!
//! What a role may allow over resources, in the three shapes
//! [`grant`](crate::daemon::grant) states: [`Make`], the action that
//! brings one into being; [`Over`], the actions over those that exist;
//! and [`Permission`], one grant's worth of either, or of tagging.

mod make;
mod over;
mod permission;

pub use make::*;
pub use over::*;
pub use permission::*;
