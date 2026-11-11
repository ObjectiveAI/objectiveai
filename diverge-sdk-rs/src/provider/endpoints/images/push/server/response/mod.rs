//! Image push response data.
//!
//! [`Frame`] is what comes back on channel `0` — a blob pulled from
//! the caller, the image pushed, the word that the caller owns it
//! already, a refusal, or a failure — and [`Blob`], [`Pushed`] and
//! [`Exists`] are what the first three carry.

mod blob;
mod exists;
mod frame;
mod pushed;

pub use blob::*;
pub use exists::*;
pub use frame::*;
pub use pushed::*;
