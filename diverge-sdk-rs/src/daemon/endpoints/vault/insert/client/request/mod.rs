//! Insert request data. What a caller hands the daemon to insert into
//! a vault key: its name, the value, and which slot to replace, if
//! any.

mod frame;

pub use frame::*;
