//! The provider half, behind the `server` feature.
//!
//! Everything else in this crate is a message. This is the part that
//! does something with one, and it is opt-in for exactly that reason:
//! a client, and a tool that only inspects the protocol, should not
//! have to compile a provider to read a frame.
//!
//! # The socket, and only the socket
//!
//! It takes a [`Connection`](crate::connection::Connection) — incoming or
//! outgoing, because a provider usually waits to be connected to and
//! sometimes connects to a caller it cannot otherwise reach, and
//! answers the same frames either way.
//!
//! What it cannot do is make one. The feature brings `axum` without
//! `http1` or `http2`, and `tokio-tungstenite` with `stream` alone —
//! enough to name both socket types and not to serve or dial with
//! either. A provider stands up its own server, or connects with its
//! own client, and hands over what comes out.
//!
//! Which keeps the choice where it belongs. Where the endpoint lives,
//! what the URL is, what authenticates the upgrade, what else that
//! server or process does — none of it is this protocol's business,
//! and a crate that served HTTP would have opinions about all of it.
//!
//! # Nothing is re-exported
//!
//! A provider that hands over a socket already depends on axum, and
//! naming `axum::extract::ws::WebSocket` through this crate would put
//! a second path on somebody else's type — one that says nothing new
//! and goes stale the day the version underneath it moves. What
//! [`connection`](crate::connection) takes, it takes by that name.
//!
//! The [`frame`](crate::frame) layer stays free of both libraries
//! regardless: it decodes `&[u8]`, because axum defines its own
//! `Message` and pins a tungstenite version of its own, and a
//! repository can easily hold two incompatible tungstenites at once —
//! this one does. A provider on some third transport keeps the frame
//! layer and leaves this module off.
//!
//! # What is here
//!
//! Two types that do something, and the whole shape is in how they
//! divide.
//!
//! [`session`] is a connection: it takes one, splits it, and answers
//! with the scopes a client opens on it. [`scope_handle`] is one of
//! those scopes: what was asked, the channels the client opens inside
//! it, and the channels this end opens back.
//!
//! The reading is all in the first and the writing is all in the
//! second. So a [`Session`](session::Session) holds the read half and
//! shares the write half out, and a
//! [`ScopeHandle`](scope_handle::ScopeHandle) carries a share of it
//! along with its own inbox.
//!
//! [`channel`] is what a scope gets back when it opens one — a number
//! and the receiver its answers arrive on. Data and nothing else, which
//! is why it sits beside the handle rather than inside it: what makes
//! one is the handle's business, and what to do with one is a caller's.
//!
//! `notice` is the queue running the other way, from the scopes back to
//! the session, carrying the two things a session cannot work out for
//! itself: where a channel's answer should land, and what has ended. It
//! is private, because a session makes both ends of it and there is
//! nothing for a caller to wire up.
//!
//! # Why it is not the caller half turned around
//!
//! [`client`](crate::client) is a router and a handle, and neither
//! shape survives the crossing. The reason is the one asymmetry in the
//! whole protocol: **a client mints scope numbers and a server only
//! ever learns them.**
//!
//! A client arranges where a scope's frames will go before it opens
//! one, so something must hold those arrangements and match arriving
//! frames against them. That is routing, and it is a job worth a type.
//! Nothing here can arrange a SCOPE — a server is told about those —
//! so what would have been a router is a loop that reads frames and
//! hands out the ones that open something, which is what a
//! [`Session`](session::Session) is.
//!
//! Channels are the other way round: a channel this end opens is this
//! end's to number, and its answer needs somewhere to land before it
//! arrives. So a session routes for the half it opens and merely
//! delivers for the half it is told about, and the one queue running
//! back to it is what carries the difference.
//!
//! The handle divides the other way for the same reason. A client's is
//! a shared, cloneable writer that hands out scopes it CREATED, and
//! every clone can create another. A provider creates none, so there is
//! nothing to share at that level: the writing belongs to the
//! individual scope, and that is where the connection's write half
//! ends up.
//!
//! # What is not here yet
//!
//! **Dispatch.** A scope can now be read as well as written —
//! [`ScopeHandle::request`](scope_handle::ScopeHandle::request) hands
//! out the payload and
//! [`recv_channel_request`](scope_handle::ScopeHandle::recv_channel_request)
//! takes the channels off their queue — and six endpoints have a
//! `server::handle` that uses them: the five
//! [`volumes`](crate::endpoints::volumes) and
//! [`version`](crate::endpoints::version). What is missing is the thing
//! in front of them: nothing
//! reads the leading tag byte of a request and decides which handler it
//! belongs to, so the pieces exist and nothing wires them together.
//!
//! **Auth**, in both directions. A credential belongs to the connection
//! and there is nowhere for one to go, so a
//! [`ClientFrame::Auth`](crate::frame::client::ClientFrame::Auth) is
//! discarded and a provider on an
//! [`Outgoing`](crate::connection::Connection::Outgoing) connection
//! cannot send the one it owes.
//!
//! There was a handler tree here before any of this, written before the
//! client half had a shape. It is gone rather than carried — what
//! answers requests should be built knowing how the reading was solved,
//! not around a sketch that predates it.
//!
//! # And what a provider supplies
//!
//! Two traits, which are what this half asks FOR rather than provides.
//! They mirror what [`client`](crate::client) has five of: something a
//! provider implements, so that the parts this crate cannot know are
//! somebody else's.
//!
//! [`container_deployer`] is the first — where a container actually
//! runs.
//!
//! It is generic across the three endpoints that put a container
//! somewhere, because they differ in what goes in one and not in how
//! one is deployed. What it is handed is a [`deployment`], and what it
//! hands back is a [`container`] — whatever that provider holds a
//! running container by, with the four things that need something only
//! the deploy learned: where its filesystem is, for reading and
//! writing; how to end it; and where to reach it, for a socket to a
//! port inside it.
//!
//! That last one is a byte pipe rather than an HTTP client, because
//! more than one protocol runs over it and always did.
//!
//! A deployment's [`mount`] is the one piece of it that is not simply
//! copied out of a request. A caller names a volume, and a name is
//! unique within the caller it was listed to — so what a handler puts
//! in a deployment is the name together with whoever it authenticated,
//! which is a fact only this half of the connection has.
//!
//! [`client_registry`] is the piece that goes with it, and it is a
//! concrete type rather than a trait: pulling an image the CALLER
//! serves means opening a channel on a scope, which is this crate's
//! machinery and not something a provider could implement. What comes
//! back on one is an [`oci_stream`].
//!
//! [`volume_manager`] is the second, and it is the other five
//! endpoints: the directories a provider offers, listed, created,
//! resized, deleted and watched. One trait for all of them, because
//! they are five verbs over one namespace rather than five subjects.
//!
//! It takes a client identity on every method, which is the same fact
//! a [`mount`] carries — a volume's name is unique within the caller it
//! was listed to, so a name alone asks a question with more than one
//! answer.
//!
//! The two traits do not know about each other. A mount reaches a
//! deployer as a name and an identity, and finding the directory is
//! the deployer's, the way publishing a port already is.
//!
//! Nothing implements either. The five
//! [`volumes`](crate::endpoints::volumes) endpoints each have a
//! `server::handle` that calls a [`volume_manager`], which is the first
//! thing in this crate to consume one of these traits; nothing calls a
//! [`container_deployer`] yet. What is missing in front of both is the
//! dispatch that reads a request's tag and picks which to call.
//!
//! [`version`](crate::endpoints::version) has a handler too and asks
//! for neither, its answer being a compile-time constant. It is the one
//! request a provider can serve without supplying anything at all.

pub mod channel;
pub mod client_registry;
pub mod container;
pub mod container_deployer;
pub mod deployment;
pub mod mount;
mod notice;
pub mod oci_stream;
pub mod scope_handle;
pub mod session;
pub mod volume_manager;
