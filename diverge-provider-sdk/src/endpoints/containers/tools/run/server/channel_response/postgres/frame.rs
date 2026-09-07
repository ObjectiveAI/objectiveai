//! What a server's response frame carries on a Postgres channel.

/// pgwire as the container wrote it, on its way to the caller's database. See [`postgres::response::Frame`](crate::shared::containers::postgres::response::Frame).
///
/// An alias rather than a re-export because this module is real: the
/// path says this scope's answer lives here, and it does, rather than
/// naming somewhere else and hoping a reader follows. What the alias
/// points at is right there in the signature.
pub type Frame<'a> = crate::shared::containers::postgres::response::Frame<'a>;
