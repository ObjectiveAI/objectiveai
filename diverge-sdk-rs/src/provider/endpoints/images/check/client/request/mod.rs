//! Image check request data.
//!
//! What a caller hands a provider to ask whether an image can be
//! supplied: the [`Frame`], the image and where to look for it, and
//! [`Sources`], the where. There is nothing to establish and nothing
//! to resume — a check is answerable from the question alone.

mod frame;
mod sources;

pub use frame::*;
pub use sources::*;
