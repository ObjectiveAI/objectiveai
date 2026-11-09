//! Grants over connected tools that name none of them.
//!
//! What a dependency tool template may ask over connected tools —
//! other daemons' standalone tools, held here — in the shapes
//! [`permission`](crate::shared::permission) states: [`Make`], the
//! action that brings one into being; [`Over`], the actions over those
//! that exist, reaching by tags; and [`Permission`], one grant's worth
//! of either, or of tagging. The daemon's grants over connected tools
//! are made of the same [`Make`] and [`Over`].

mod make;
mod over;
mod permission;

pub use make::*;
pub use over::*;
pub use permission::*;
