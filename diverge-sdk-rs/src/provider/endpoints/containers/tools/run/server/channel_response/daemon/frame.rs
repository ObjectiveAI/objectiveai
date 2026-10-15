//! What a server's response frame carries on a daemon channel.

/// One client frame the program sent on the connection, on its way to
/// the daemon. See
/// [`daemon::client::Frame`](crate::shared::containers::daemon::client::Frame).
///
/// An alias rather than a re-export because this module is real: the
/// path says this scope's answer lives here, and it does, rather than
/// naming somewhere else and hoping a reader follows. What the alias
/// points at is right there in the signature.
pub type Frame<'a> = crate::shared::containers::daemon::client::Frame<'a>;
