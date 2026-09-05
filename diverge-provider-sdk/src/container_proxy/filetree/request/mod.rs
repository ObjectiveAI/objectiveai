//! What the container sends on `/filetree`: one filetree event per
//! frame. The server sends nothing, so there is no `response`.

mod frame;

pub use frame::*;
