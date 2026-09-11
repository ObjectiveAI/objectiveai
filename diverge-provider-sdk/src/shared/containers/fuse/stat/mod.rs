//! A mounted entry, described: the mount and the path, answered with
//! one message — what it is and how long, or that there is nothing
//! there — then the close. The ask a file's length or a path's
//! existence costs, instead of the bytes or the parent's listing.

pub mod request;
pub mod response;

mod stat;

pub use stat::*;
