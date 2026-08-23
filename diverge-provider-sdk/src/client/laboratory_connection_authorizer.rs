//! Deciding who may attach to a laboratory.

use std::future::Future;

use crate::endpoints::laboratories::run::server::channel_request::Authorize;

/// What decides whether a connector may join a running laboratory.
///
/// A provider asks this when somebody arrives at a container's door. It
/// does not judge the request itself and could not usefully: what makes
/// one connector acceptable is something only the runner knows, so the
/// credential is relayed verbatim and the answer comes back here.
///
/// # It is not a proxy, and is not named one
///
/// The four in [`client`](crate::client) that are — an
/// [`McpProxy`](super::mcp_proxy::McpProxy), an
/// [`OciProxy`](super::oci_proxy::OciProxy), a
/// [`PostgresProxy`](super::postgres_proxy::PostgresProxy), a
/// [`CommandProxy`](super::command_proxy::CommandProxy) — each stand in
/// front of something real and forward to it. There is nothing on the
/// other side of this one. It decides, and the decision is the whole
/// answer.
///
/// # Which makes it the first whose answer is not an aside
///
/// Every proxy punts failure into a vocabulary that already exists — an
/// HTTP status, a pgwire `ErrorResponse`, an item shaped however the
/// CLI shapes one — and none of them has an error variant, because a
/// second way to say a thing is a second thing to disagree about.
///
/// Here "no" is the point. [`Decision`] is not an error path beside the
/// real answer; it IS the answer, and a denial is as ordinary an
/// outcome as an admission.
///
/// # A connector is waiting on this
///
/// Which is true of nothing else in this module. The frame guarantees
/// that the question is answered before the connection it is about is
/// allowed to open, so an implementation that takes its time is holding
/// somebody's connection open while it does.
///
/// A decision that needs to consult something slow should consult it
/// and answer, rather than answering provisionally — there is no way to
/// change the answer afterwards, and nothing here will ask twice.
///
/// # It names an endpoint's type, which two things here now do
///
/// [`Authorize`] belongs to
/// [`laboratories::run`](crate::endpoints::laboratories::run), and
/// [`PostgresProxy`](super::postgres_proxy::PostgresProxy) set the
/// precedent for reaching for one. The reason is the same: what is
/// being decided is that endpoint's question, and no shared type says
/// it.
///
/// Taking the struct rather than its two fields loose is also the safer
/// shape.
/// [`address`](Authorize::address) is ATTESTED — the provider saw it on
/// a socket — and
/// [`authorization`](Authorize::authorization) is ASSERTED — the
/// connector wrote it and nobody checked. Its own documentation calls
/// confusing the two "the whole of how this kind of check gets
/// defeated", and two arguments side by side are easier to confuse than
/// two named fields.
pub trait LaboratoryConnectionAuthorizer: Send + Sync {
    /// Decide one connector.
    ///
    /// # The future is [`Send`]
    ///
    /// Because several connectors may arrive at once, at one laboratory
    /// or at several a runner holds, and a decision that cannot move
    /// between threads pins all of them to one. It is spelled out
    /// rather than left to `async fn`, which promises nothing about the
    /// future it returns.
    ///
    /// # The request is owned
    ///
    /// Unlike an [`McpProxy`](super::mcp_proxy::McpProxy) method,
    /// which borrows because it is answered in the task that received
    /// the frame. An [`Authorize`] carries an owned
    /// [`String`] already — a JSON credential with an escape in it is
    /// not a slice of the bytes it arrived in — so there is nothing to
    /// borrow and nothing saved by trying.
    fn handle(
        &self,
        request: Authorize,
    ) -> impl Future<Output = Decision> + Send;
}

/// Yes or no, and if yes, what to call it.
///
/// # The name is for the runner's own benefit
///
/// It comes back on a
/// [`Disconnected`](crate::endpoints::laboratories::run::server::response::Disconnected)
/// when this connector leaves, and that is its entire purpose: a runner
/// that authorized four connectors has no other way to learn WHICH one
/// left. The provider stores it and hands it back; it never reads it,
/// and the connector is never told it has one.
///
/// # It is not optional, and may be empty
///
/// A runner that does not care to tell its connectors apart names them
/// all the same thing, and the empty string is the obvious one. Their
/// departures then arrive under a name that distinguishes nothing —
/// which is exactly what an absent name said, with one fewer case for
/// everyone downstream to hold.
///
/// Nothing requires it to be unique either. Two connectors may share a
/// nickname, and a departure then names both and resolves neither,
/// which is the runner's own arrangement rather than something this
/// protocol did to it.
///
/// # Owned, because this end makes it up
///
/// The frame it becomes carries `&str`, borrowed from the bytes it was
/// decoded out of. Nothing is being decoded here — an implementation
/// invents the name — so there is nothing for a borrow to point at.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Decision {
    /// The connector may not attach.
    ///
    /// It never joins, and nothing further is said to it or about it.
    /// There is no [`Disconnected`](crate::endpoints::laboratories::run::server::response::Disconnected)
    /// for a connector that never arrived.
    Denied,
    /// The connector may attach, under this name.
    Authorized(String),
}
