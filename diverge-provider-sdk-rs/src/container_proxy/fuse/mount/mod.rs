//! `/fuse/mount`: one mount, made on the server's request.
//!
//! The one path under `/fuse/` the server opens rather than answers.
//! It sends one message, the [`request::Request`] — the path, the
//! id, and which kind: a file or a tree —
//! and the proxy mounts, at that path, a FUSE filesystem whose
//! contents are the caller's, asked by the id over the six siblings.
//! The answer is one [`response::Frame`], sent only once the mount
//! is complete: `Ok`, the mount serving; or `Error` with why it was
//! not made — the path empty or the root, a path the proxy already
//! mounted, a mount point it could not make, no FUSE on the host, a
//! session that would not start. Then the close.
//!
//! ```text
//! server → container:   [request JSON]           once
//! container → server:   [0] | [1][message…]      once the mount is complete, then the close
//! ```
//!
//! A close before the request, or a request that will not decode,
//! is the clean close with nothing before it. A mount lives for the
//! proxy's life; nothing unmounts one. An abrupt end is a mount whose
//! fate the server did not hear, and nothing retries it.
//!
//! # Before everything
//!
//! The server mounts before it asks anything else of the container:
//! every mount is complete before the agent is registered and before
//! any `/filesystem/tree` is opened, so what a program finds at the
//! path is the mount and never the image's own file, and what a tree
//! reports never includes one.

pub mod request;
pub mod response;

#[cfg(feature = "server")]
pub mod execute;
