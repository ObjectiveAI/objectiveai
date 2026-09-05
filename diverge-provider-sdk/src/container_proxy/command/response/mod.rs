//! What the server answers on `/command`: the frame, and the item or
//! error inside it.

mod frame;
mod response;

pub use frame::*;
pub use response::*;
