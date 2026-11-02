//! What a client's channel request frame carries for a terminal: its
//! input, as [`terminal`](crate::shared::containers::terminal) states it.

pub use crate::shared::containers::terminal::input::{Input as Frame, InputError as FrameError};
