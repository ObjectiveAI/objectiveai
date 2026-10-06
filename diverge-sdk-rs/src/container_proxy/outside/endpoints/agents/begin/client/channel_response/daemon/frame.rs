//! What a client's response frame carries on a daemon channel.

/// One server frame of the program's daemon connection, or the error
/// that says the client frame was not served. See
/// [`daemon::response::Frame`](crate::shared::containers::daemon::response::Frame).
///
/// An alias rather than a re-export because this module is real: the
/// path says this scope's answer lives here, and it does.
pub type Frame<'a> = crate::shared::containers::daemon::response::Frame<'a>;
