//! Uploading a resource: one file, or one directory of files.
//!
//! A client opens the scope saying which kind, what the resource is
//! in words, and for a directory which files; the daemon asks for
//! each file's content on a channel
//! of its own, the client answers each in pieces and finishes it, and
//! when every channel has finished the daemon answers once with the
//! resource's id — new, or held already, in which case the
//! description given is the resource's from then on — or a failure,
//! and the scope finishes.
//!
//! Split by who SENDS, as everywhere else. The kind and the content
//! are in [`client`]; the asks for the content and the answer are in
//! [`server`].
//!
//! # Why the content travels on channels the daemon opens
//!
//! Because only a responder can end a channel. A client streaming
//! content into its own scope would have to invent an end marker; a
//! channel the daemon opens is answered by the client and finished by
//! the client, which is the end. The provider protocol's volume write
//! is the same exchange for one file, and this is that exchange, as
//! many times as there are files.

pub mod client;
pub mod server;
