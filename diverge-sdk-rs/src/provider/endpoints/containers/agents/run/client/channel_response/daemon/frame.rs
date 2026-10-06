//! What a client's response frame carries on a daemon channel.

/// One server frame of the container's daemon connection, or the error
/// that says the client frame was not served. See
/// [`daemon::response::Frame`](crate::shared::containers::daemon::response::Frame).
///
/// An alias rather than a re-export because this module is real: the
/// path says this scope's answer lives here, and it does, rather than
/// naming somewhere else and hoping a reader follows. What the alias
/// points at is right there in the signature.
pub type Frame<'a> = crate::shared::containers::daemon::response::Frame<'a>;
