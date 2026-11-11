//! What the proxy's response frames carry for a frontend request: the
//! program's answer, as [`frontend`](crate::shared::containers::frontend) states it.

pub use crate::shared::containers::frontend::output::{Frame, FrameError, Owned};
