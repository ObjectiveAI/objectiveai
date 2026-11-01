//! An exec channel's responses: the command's stream, as
//! [`exec`](crate::shared::containers::exec) states it, relayed from the
//! proxy as it is.

pub use crate::shared::containers::exec::response::{Frame, FrameError, Owned};
