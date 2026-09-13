//! What a server's response frame carries for a FUSE mount.

/// `Ok` once the mount is complete — the mount point made, the
/// session started, the path serving — and then the scope stays open
/// for the proxy's life; or `Error` with why not, then the finish:
/// the path empty or the root, a path the proxy already mounted, a
/// mount point it could not make, no FUSE on the host, a session that
/// would not start. See [`ack::Frame`](crate::shared::containers::fuse::ack::Frame).
///
/// An alias rather than a re-export because this module is real: the
/// path says this scope's answer lives here, and it does, rather than
/// naming somewhere else and hoping a reader follows. What the alias
/// points at is right there in the signature.
pub type Frame<'a> = crate::shared::containers::fuse::ack::Frame<'a>;
