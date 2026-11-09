//! Grants over the daemons reached through providers, naming no
//! provider.
//!
//! What a grant may ask through the daemon's providers of the daemons
//! connected to them, in the shape
//! [`permission`](crate::shared::permission) states for actions over
//! what exists: [`Over`], the actions — listing the listable identities,
//! listening for messages, sending one — reaching the providers by their
//! tags, as a grant over [`providers`](super::providers) does; and
//! [`Permission`], one grant's worth of them. Nothing is made and
//! nothing is tagged here: the records are the providers', and the
//! providers grant makes and tags them.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod over;
mod permission;

pub use over::*;
pub use permission::*;
