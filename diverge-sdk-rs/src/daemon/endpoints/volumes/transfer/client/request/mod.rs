//! Transfer request data. What a caller hands the daemon to transfer:
//! which volume, the path in it, and where it lands.

mod frame;

pub use frame::*;
