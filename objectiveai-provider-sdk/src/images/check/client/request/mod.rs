//! Image check request data.
//!
//! What a caller hands a provider to ask whether an image can be
//! supplied. There is nothing to establish and nothing to resume — a
//! check is answerable from the question alone, which is why this is
//! one type and not a module of them.

mod frame;

pub use frame::*;
