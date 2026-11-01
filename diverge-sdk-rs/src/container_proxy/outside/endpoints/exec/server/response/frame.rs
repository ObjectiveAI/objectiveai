//! What the proxy's response frames carry for an exec: the command's
//! stream, as [`exec`](crate::shared::containers::exec) states it.

pub use crate::shared::containers::exec::response::{Frame, FrameError, Owned};
