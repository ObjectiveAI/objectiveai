//! What a client's response frame carries on an OCI channel.

/// One registry answer: the head once, then as much body as there
/// turns out to be.
///
/// The body is a manifest, a config blob, or a layer — which can run
/// to hundreds of megabytes, and is the reason the head goes first.
/// A provider writes the status and headers onto the runtime's socket
/// the moment it has them and pumps the rest through, rather than
/// holding a layer in memory to learn its length.
///
/// See [`http::response::Frame`](crate::shared::http::response::Frame).
pub type Frame<'a> = crate::shared::http::response::Frame<'a>;
