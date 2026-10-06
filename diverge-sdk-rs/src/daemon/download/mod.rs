//! What a download sends: one piece of one file, with the file's path.
//!
//! A download — an
//! [agent's](crate::daemon::endpoints::agents::download), a
//! [tool's](crate::daemon::endpoints::tools::download), a
//! [resource's](crate::daemon::endpoints::resources::download) — is a
//! stream of [`Chunk`]s, each a piece of one file and the path of that
//! file relative to what was asked for, so that a reader writes every
//! chunk at its destination joined with the chunk's path and needs no
//! other state. The chunk is one shape for the three, defined here and
//! wrapped by each family's response frame.

mod chunk;

pub use chunk::*;
