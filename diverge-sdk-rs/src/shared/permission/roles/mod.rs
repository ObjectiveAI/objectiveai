//! Grants over roles that name none of them.
//!
//! What a dependency tool template may ask over roles, in the
//! shapes [`permission`](crate::shared::permission) states: [`Make`],
//! the actions that bring one into being; [`Over`], the actions over
//! those that exist, reaching by tags; and [`Permission`], one grant's
//! worth of either, or of tagging. The daemon's grants over roles
//! are made of the same [`Make`] and [`Over`].

mod make;
mod over;
mod permission;

pub use make::*;
pub use over::*;
pub use permission::*;
