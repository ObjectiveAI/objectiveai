//! What a client's response frame carries on a command channel.

/// One command's answer: the head once, then as much body as there
/// turns out to be.
///
/// An alias, because a command channel carries a tunneled HTTP exchange
/// and the answer to one is that exchange's response. There is nothing
/// command-shaped about a status and some headers followed by a body,
/// so there is nothing here to define — see
/// [`http::response::Frame`](crate::shared::http::response::Frame) for
/// what it is and why it is split.
///
/// An alias rather than a re-export because this module is real. The
/// path says a command channel's response lives here, and it does,
/// rather than naming somewhere else and hoping a reader follows. What
/// the alias points at is right there in the signature.
///
/// # It used to be items
///
/// One frame per thing the command produced, opaque, with the channel's
/// finish as the only terminator and no head at all. Which worked, and
/// cost the two things an envelope gives for nothing: a command that
/// failed had to say so in an item a reader was expected to recognise,
/// and a command that answered once was still a stream, because one
/// command somewhere was.
///
/// A status says the first. A body of one piece says the second. The
/// body is still opaque — what a command produces belongs to the CLI —
/// so nothing was given up by putting an envelope around it.
pub type Frame<'a> = crate::shared::http::response::Frame<'a>;
