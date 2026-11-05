//! Grants over remote daemons that name none of them.
//!
//! The remote daemons are the daemon's records of them, each one account
//! of ours on another daemon. What a dependency tool template may ask
//! over them, in the shapes [`permission`](crate::shared::permission)
//! states: [`Make`], the action that records one; [`Over`], the actions
//! over the records — reading, changing, deleting, and registering a
//! connected tool on one or relaying to one — reaching by tags; and
//! [`Permission`], one grant's worth of either, or of tagging.

mod make;
mod over;
mod permission;

pub use make::*;
pub use over::*;
pub use permission::*;
