//! What the client answers a content channel with: the file's bytes
//! in pieces, or the news that it cannot.
//!
//! [`Frame`] is the provider protocol's own answer to a volume
//! write's content ask, which is the same exchange: a body piece of
//! at most [`CHUNK_SIZE`](crate::CHUNK_SIZE), appended to the pieces
//! before it, or an error that abandons the upload. A file of zero
//! bytes is one empty piece. The channel's finish is the end of the
//! file.

mod frame;

pub use frame::*;
