//! What a client's channel request frame carries for a frontend
//! request: its input, as [`frontend`](crate::shared::containers::frontend) states it.

pub use crate::shared::containers::frontend::input::{Input as Frame, InputError as FrameError};
