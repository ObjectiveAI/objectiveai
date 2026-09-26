//! Which side of a connection authenticates, and with what.

/// What a provider brings to a connection's handshake.
///
/// The argument [`handle`](crate::provider::server::handle::handle) takes where
/// `client_identity` used to be, and the fact it encodes is the one the
/// old parameter hid: WHERE an identity comes from depends on which
/// side dialled. Whichever side dialled authenticates — that is the
/// wire's rule, stated on [`Auth`](crate::wire::frame::auth::Auth) — so a
/// connection has one credential, and which end presents it is a fact
/// about the socket.
///
/// # [`Incoming`](Self::Incoming): the peer proves itself
///
/// The peer dialled us, so the peer authenticates: its first frame
/// must be the credential, and the
/// [`UnbrokeredAuthorizer`](super::unbrokered_authorizer::UnbrokeredAuthorizer)
/// carried here is what judges it. The identity is whatever the
/// authorizer says it is.
///
/// # [`Outgoing`](Self::Outgoing): we prove ourselves
///
/// We dialled the peer, so we authenticate: the [`Auth`] carried here
/// goes out as the connection's first frame, before anything is read.
/// And the identity rides beside it, because the provider DIALLED this
/// peer — who it is is a fact of the dial, established when the
/// provider chose whom to call, and no frame could tell it anything it
/// does not already know.
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
    /// The peer dialled us: its first frame must be the credential,
    /// and this judges it.
    Incoming {
        /// What judges an
        /// [`Unbrokered`](crate::wire::frame::auth::Auth::Unbrokered)
        /// credential.
        ///
        /// Named for the mode it judges, because a Brokered mode is
        /// coming — see [`Auth`] — and it will be judged by something
        /// else, sitting beside this rather than replacing it.
        unbrokered: U,
    },
    /// We dialled the peer: this credential goes out first, and the
    /// identity is already known.
    Outgoing {
        /// The credential to present, as the connection's first frame.
        auth: Auth,
        /// Who the peer is.
        ///
        /// What every handler and every capability trait receives, the
        /// same opaque string an
        /// [`UnbrokeredAuthorizer`](super::unbrokered_authorizer::UnbrokeredAuthorizer)
        /// produces on the other variant. Supplied rather than derived:
        /// the provider chose whom to dial, and that choice IS the
        /// identification.
        client_identity: String,
    },
}

/// A credential to present, owned.
///
/// The argument-side mirror of [`frame::auth::Auth`](crate::wire::frame::auth::Auth),
/// which borrows from the frame it was decoded out of and cannot ride
/// an argument into a spawned connection. This one owns its string and
/// travels; the wire type is what it becomes at the socket.
///
/// One variant today, and an enum anyway, for the reason the wire type
/// spends a mode byte on one mode: a Brokered credential is coming —
/// see `diverge-broker-sdk` — and it will be a second variant here the
/// day it is tag `1` there. An argument that started as a bare string
/// would make that day a signature break on every provider.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Auth {
    /// The two ends already know each other, and this is the
    /// credential they agreed on.
    ///
    /// Everything [`frame::auth::Auth::Unbrokered`](crate::wire::frame::auth::Auth::Unbrokered)
    /// says about what may be in the string holds here, since this is
    /// that, one copy earlier.
    Unbrokered(String),
}
