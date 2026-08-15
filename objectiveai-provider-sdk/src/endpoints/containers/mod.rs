//! Containers — running an image a provider can supply.
//!
//! [`images`](crate::endpoints::images) settles whether an image is
//! available; this is what happens once it is. The two are
//! deliberately separate round trips: a caller that cannot get an
//! image has no use for the terms of running it, and asking about
//! both at once would make the common answer — no — carry a payload
//! nobody reads.
//!
//! [`laboratories`] is the one kind of container so far. Grouping it
//! rather than putting its two scopes here directly says what they
//! are ABOUT: not containers in general, but the kind an agent works
//! inside.

pub mod laboratories;
