//! Grants over tools.
//!
//! What a role may allow over tools, in the three shapes
//! [`grant`](crate::daemon::grant) states: [`Make`], the actions that
//! bring one into being; [`Over`], the actions over those that exist;
//! and [`Permission`], one grant's worth of either, or of tagging.

mod make;
mod over;
mod permission;

pub use make::*;
pub use over::*;
pub use permission::*;
