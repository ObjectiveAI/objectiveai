//! Image build response data.
//!
//! [`Frame`] is what comes back on channel `0` — a line of the
//! build's output, the image built, the word that there was no room,
//! or a failure — and [`Built`] is what the image built carries.

mod built;
mod frame;

pub use built::*;
pub use frame::*;
