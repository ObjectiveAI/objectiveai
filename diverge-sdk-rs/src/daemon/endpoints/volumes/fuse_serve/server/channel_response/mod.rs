//! What the daemon sends back on the channels a caller opens on its
//! FUSE serve scope: the provider's serve's own answers, one module
//! per kind, as the provider answered them. The cancel has no answer.
//!
//! Re-exported whole rather than aliased one by one: there are ten
//! kinds, each the provider's, and the path says where this scope's
//! answers are found.

pub use crate::provider::endpoints::volumes::fuse_serve::server::channel_response::{filetree, list, mkdir, read, remove, rename, setattr, stat, truncate, write};
