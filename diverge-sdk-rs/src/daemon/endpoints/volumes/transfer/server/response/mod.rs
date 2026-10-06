//! The transfer response: landed, with a new resource's id when there
//! is one, no such volume or path, no such destination, a volume held,
//! forbidden, or a failure.

mod frame;

pub use frame::*;
