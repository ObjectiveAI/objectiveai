//! Which side of a connection authenticates, and with what.

/// What a caller brings to a connection's handshake.
///
/// The argument [`authorize`](super::authorize::authorize) takes, and
/// the mirror of the server half's
/// [`Authorization`](crate::server::authorization::Authorization) with
/// the dial direction inverted. Whichever side dialled authenticates
/// — the wire's rule, stated on [`Auth`](crate::frame::auth::Auth) —
/// so a connection has one credential, and which end presents it is
/// a fact about the socket. And WHERE the provider's identity comes
/// from depends on the same fact.
///
/// # [`Outgoing`](Self::Outgoing): we prove ourselves
///
/// The ordinary case — a caller usually dials its provider. We dialled,
/// so we authenticate: the [`Auth`] carried here goes out as the
/// connection's first frame, before anything else. And the identity
/// rides beside it, because the caller DIALLED this provider — who it
/// is is a fact of the dial, established when the caller chose whom
/// to call, and no frame could tell it anything it does not already
/// know.
///
/// # [`Incoming`](Self::Incoming): the provider proves itself
///
/// The provider dialled us, so it authenticates: its first frame must
/// be its credential, and the
/// [`UnbrokeredAuthorizer`](super::unbrokered_authorizer::UnbrokeredAuthorizer)
/// carried here is what judges it. The identity is whatever the
/// authorizer says it is.
///
/// # Why the variants are not symmetric
///
/// One carries a judge and the other carries a credential and a
/// verdict, because the two ends of one handshake are doing opposite
/// jobs. A symmetric type would imply a mutual exchange the wire does
/// not have: one credential per connection, presented by the side that
/// dialled, and nothing coming back.
#[derive(Debug)]
pub enum Authorization<U> {
    /// The provider dialled us: its first frame must be its
    /// credential, and this judges it.
    Incoming {
        /// What judges an
        /// [`Unbrokered`](crate::frame::auth::Auth::Unbrokered)
        /// credential.
        ///
        /// Named for the mode it judges, because a Brokered mode is
        /// coming — see [`Auth`] — and it will be judged by something
        /// else, sitting beside this rather than replacing it.
        unbrokered: U,
    },
    /// We dialled the provider: this credential goes out first, and
    /// the identity is already known.
    Outgoing {
        /// The credential to present, as the connection's first frame.
        auth: Auth,
        /// Who the provider is.
        ///
        /// What [`authorize`](super::authorize::authorize) hands back
        /// beside the connection, the same opaque string an
        /// [`UnbrokeredAuthorizer`](super::unbrokered_authorizer::UnbrokeredAuthorizer)
        /// produces on the other variant. Supplied rather than
        /// derived: the caller chose whom to dial, and that choice IS
        /// the identification — the address it dialled, as often as
        /// not.
        provider_identity: String,
    },
}

/// A credential to present, owned.
///
/// The argument-side mirror of [`frame::auth::Auth`](crate::frame::auth::Auth),
/// which borrows from the frame it was decoded out of and cannot ride
/// an argument into a spawned connection. This one owns its string and
/// travels; the wire type is what it becomes at the socket.
///
/// One variant today, and an enum anyway, for the reason the wire type
/// spends a mode byte on one mode: a Brokered credential is coming —
/// see `diverge-broker-sdk` — and it will be a second variant here the
/// day it is tag `1` there. An argument that started as a bare string
/// would make that day a signature break on every caller.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Auth {
    /// The two ends already know each other, and this is the
    /// credential they agreed on.
    ///
    /// Everything [`frame::auth::Auth::Unbrokered`](crate::frame::auth::Auth::Unbrokered)
    /// says about what may be in the string holds here, since this is
    /// that, one copy earlier.
    Unbrokered(String),
}
