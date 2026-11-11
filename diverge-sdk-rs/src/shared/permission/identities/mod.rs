//! Grants over the host's identities that name none of them.
//!
//! The identities are the files under the root's `identities/`, each
//! one subject at one issuer, signed in on the host; the daemon keeps
//! their tags. What a dependency tool template may ask over them, in
//! the shapes [`permission`](crate::shared::permission) states:
//! [`Make`], the action that brings one onto the host; [`Over`], the
//! actions over those there, reaching by tags; and [`Permission`], one
//! grant's worth of either, or of tagging. The daemon's grants over
//! identities are made of the same [`Make`] and [`Over`]. No token is
//! ever answered.

mod make;
mod over;
mod permission;

pub use make::*;
pub use over::*;
pub use permission::*;
