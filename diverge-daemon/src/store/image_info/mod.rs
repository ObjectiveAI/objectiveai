//! What the daemon has learnt of each image's `/DIVERGE.md`, by
//! digest.
//!
//! A digest is the image, so what an image ships at `/DIVERGE.md` is
//! fixed by it: an answer found once is the answer for good, and
//! nothing here is ever updated or removed. [`by_digest`] reads what
//! is on record; [`save`] puts an answer on record.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod load;
mod save;

pub use load::*;
pub use save::*;
