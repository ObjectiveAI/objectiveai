//! Grants over the vault's keys that name none of them.
//!
//! What a dependency tool template may ask over the vault, in the
//! shapes [`permission`](crate::shared::permission) states: [`Make`],
//! the one action that brings a key into being; [`Over`], the actions
//! over keys that exist, reaching by tags; and [`Permission`], one
//! grant's worth of either, or of tagging. The daemon's grants over
//! the vault are made of the same [`Make`] and [`Over`].

mod make;
mod over;
mod permission;

pub use make::*;
pub use over::*;
pub use permission::*;
