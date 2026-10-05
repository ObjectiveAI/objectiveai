//! Grants over judges.
//!
//! What a role may allow over judges, in the three shapes
//! [`grant`](crate::daemon::grant) states: [`Make`], the actions that
//! bring one into being; [`Over`], the actions over those that exist;
//! and [`Permission`], one grant's worth of either.

mod make;
mod over;
mod permission;

pub use make::*;
pub use over::*;
pub use permission::*;
