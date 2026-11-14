//! What the daemon asks a client for while a provider takes a pushed
//! image from it: the provider push's own asks — a manifest, a blob,
//! the proof — byte for byte, since the daemon opens each on the
//! client's scope as the provider wrote it. See
//! [`Frame`] for the three and the order they come in.

pub use crate::provider::endpoints::images::push::server::channel_request::{Frame, FrameError};
