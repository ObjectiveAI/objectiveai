//! Reading a log, and watching it: the one `logs` endpoint.
//!
//! The request names its subject — an agent, a standalone tool or a
//! dependency tool. [`handle`] judges the request by that subject's own
//! family's `logs` grant, finds whose log it is and subscribes to it;
//! [`send`] then runs the filter over the log as it stands, oldest
//! first, one frame per item, and — watching — over each item as it
//! lands, until nothing more can come. The rules:
//!
//! 1. Every item is sent once: the historical read ends at an index,
//!    and the watch begins at the next. The endpoint subscribes before
//!    the first read, so an append between the read and the wait is not
//!    missed.
//! 2. The scope's sending ends when the count is met, when the log
//!    reaches `logs_index_to`, when an item later than `created_to`
//!    lands or the clock passes it, when the client cancels, or when
//!    the log's owner is deleted — its live sender gone.
//! 3. Without `watch`, the sending ends after the historical read.
//! 4. Every item goes out wrapped in its subject's kind.
//!
//! Its own files are flattened into it, so everything is named through
//! this module and not through the file it lives in.

mod failure;
mod handle;
mod send;

pub use failure::*;
pub use handle::*;
pub use send::*;
