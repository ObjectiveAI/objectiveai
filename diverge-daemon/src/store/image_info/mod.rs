//! What the daemon has learnt of each image's `/DIVERGE.md`, by
//! digest.
//!
//! A digest is the image, so what an image ships at `/DIVERGE.md` is
//! fixed by it: a document found once is the document for good, and
//! nothing here is ever updated or removed. Only a document is
//! recorded: an image that ships none, or one that does not parse,
//! has no row, and is asked again. [`by_digest`] reads what is on
//! record; [`save`] puts a document on record.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod load;
mod save;

pub use load::*;
pub use save::*;
