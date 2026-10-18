//! Grants over dependency templates that name none of them.
//!
//! What a dependency tool template may ask over dependency templates
//! and the tools running from them, in the shapes
//! [`permission`](crate::shared::permission) states: [`Over`], the
//! actions over those that exist, reaching by tags; and
//! [`Permission`], one grant's worth of them, or of tagging. There is
//! nothing to make: a dependency template comes to be by an agent's
//! registration, which no grant over this kind allows or refuses. The
//! daemon's grants over dependency templates are made of the same
//! [`Over`].

mod over;
mod permission;

pub use over::*;
pub use permission::*;
