//! A list stream, and the list a consumer keeps from it.
//!
//! Every list of the daemon's, and the provider's two, is a stream:
//! an initial listing, each item as added and then the word that the
//! listing is whole, and from then on each item come to be listed,
//! listed still but differing, or listed no more, until the client
//! cancels. What such a stream tells is one [`Change`] at a time —
//! the same enum the daemon's server side diffs a listing into — and
//! what a consumer keeps from it is a [`Listing`]: the list as it
//! stands, folded by [`Listing::push`] the way a filetree's root is
//! folded by [`Root::update`](crate::shared::filetree::response::Root::update).
//! The fold tells one item from another by its [`Keyed`] key, which
//! each listed record states for itself, beside its definition.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod change;
mod keyed;
mod listing;

pub use change::*;
pub use keyed::*;
pub use listing::*;
