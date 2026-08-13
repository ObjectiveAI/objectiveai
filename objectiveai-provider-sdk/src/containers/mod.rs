//! Containers — running an image a provider can supply.
//!
//! [`images`](crate::images) settles whether an image is available;
//! this is what happens once it is. The two are deliberately separate
//! round trips: a caller that cannot get an image has no use for the
//! terms of running it, and asking about both at once would make the
//! common answer — no — carry a payload nobody reads.
//!
//! Under construction. [`create`] has its shape and none of its types.

pub mod create;
