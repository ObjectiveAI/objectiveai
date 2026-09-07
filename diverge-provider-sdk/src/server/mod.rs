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
//! It does not speak HTTP either, in either direction. It did twice
//! over: a container was reached through a byte pipe, so something here
//! had to turn one into a request; and then through parsed requests and
//! statuses and header maps, which only moved the problem up a level.
//!
//! Now [`Container`](container::Container) takes and returns the
//! exchanges themselves — MCP asks and MCP answers — and whatever
//! speaks HTTP to a container is on the provider's side of that line
//! with everything else about how a container is reached. Which is
//! where it belongs: an implementation is already an MCP client and an
//! MCP server, and neither of those is a thing this crate should be
//! reimplementing halfway.
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
//! Sharing happens one level down instead. A
//! [`ScopeHandle`](scope_handle::ScopeHandle) takes `&self` throughout,
//! so several tasks can answer one scope at once — which is what a
//! provider needs where a client does not, because a
//! [`containers`](crate::endpoints::containers) scope writes in two
//! directions and neither should wait on the other. A client shares
//! the thing that makes scopes; a provider shares the scope.
//!
//! # Every endpoint is handled, and [`handle`] wires them together
//!
//! This module's [`handle`] is the dispatch in front of the endpoint
//! handlers: it reads each request a [`Session`](session::Session)
//! yields, decides which endpoint it belongs to, and spawns that
//! endpoint's handler with the decoded request. One call per
//! connection is a provider's whole loop.
//!
//! The five [`volumes`](crate::endpoints::volumes),
//! [`images::check`](crate::endpoints::images::check) and
//! [`version`](crate::endpoints::version) answer and finish, which is
//! the whole of what those endpoints do. The four
//! [`containers`](crate::endpoints::containers) scopes serve for as
//! long as their containers run; their handlers are not written yet,
//! and until they are the dispatch finishes those scopes with nothing.
//!
//! # Auth is a handshake in front of [`handle`]
//!
//! Whichever side dialled authenticates. On an
//! [`Incoming`](crate::connection::Connection::Incoming) connection the
//! peer's first frame must be its credential, judged by the
//! [`unbrokered_authorizer`] the provider supplies — the identity it
//! produces is the `client_identity` everything downstream receives.
//! On an [`Outgoing`](crate::connection::Connection::Outgoing)
//! connection this end sends the credential before reading anything,
//! and the identity was never in question: the provider dialled the
//! peer, so it already knows who it is. [`authorization`] is the
//! argument that says which, and [`handle`]'s error type is where a
//! handshake that failed is reported — never to the peer, which is the
//! auth frame's own no-answer rule.
//!
//! The client half tells the same story from the other chair:
//! [`client::authorize`](crate::client::authorize) presents this
//! end's credential or judges a provider's, and the two handshakes
//! are mirrors with one asymmetry — only this half derives an
//! identity, because only this half acts on somebody's behalf.
//!
//! # And what a provider supplies
//!
//! Four traits, which are what this half asks FOR rather than
//! provides. They mirror the ones [`client`](crate::client) supplies:
//! something a provider implements, so that the parts this crate cannot
//! know are somebody else's.
//!
//! [`container_deployer`] is the first — where a container actually
//! runs.
//!
//! It is generic across the three endpoints that put a container
//! somewhere, because they differ in what goes in one and not in how
//! one is deployed. What it is handed is a [`deployment`] and the
//! caller it is for — the same opaque identity a [`mount`] carries,
//! because deploying is the most consequential thing a provider does on
//! somebody's behalf and every question worth asking about it needs to
//! know whose. What it
//! hands back is a [`container`] — whatever that provider holds a
//! running container by, with the things that need something only the
//! deploy learned: where its filesystem is, for reading and writing,
//! and how to end it.
//!
//! Reaching a port inside one belonged here too and is gone for now,
//! while the shape of it is reconsidered. It will come back as
//! something: a container nothing can speak to serves nobody.
//!
//! A deployment's [`mount`] is the one piece of it that is not simply
//! copied out of a request. A caller names a volume, and a name is
//! unique within the caller it was listed to — so what a handler puts
//! in a deployment is the name together with whoever it authenticated,
//! which is a fact only this half of the connection has.
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
//! [`image_checker`] is the third, and it is the smallest: would you
//! supply this image, to this caller. It is not a
//! [`container_deployer`] method because a check creates nothing and
//! pulls nothing, and because no is an ANSWER here where it would be an
//! error there.
//!
//! [`unbrokered_authorizer`] is the fourth, and it is the door: every
//! other trait is asked things on behalf of a caller, and this is the
//! one that decides who the caller IS. It is consumed by [`handle`]
//! itself rather than by any endpoint's handler, because a credential
//! belongs to the connection and not to any scope on it.
//!
//! Nothing implements any of them, and all four are consumed:
//! [`volume_manager`] by the five
//! [`volumes`](crate::endpoints::volumes) endpoints' handlers,
//! [`image_checker`] by
//! [`images::check`](crate::endpoints::images::check)'s, and
//! [`container_deployer`] by the two run handlers of
//! [`containers`](crate::endpoints::containers) — the scopes that put
//! a container somewhere, which use all three of its methods. A
//! connect handler consumes none of the three, because it deploys
//! nothing — the container it serves already exists, and is stopped by
//! whoever ran it.
//!
//! [`version`](crate::endpoints::version) has a handler too and asks
//! for nothing at all, its answer being a compile-time constant. It is
//! the one request a provider can serve without supplying anything.
//!
//! And in front of all of them stands [`handle`]: take a
//! [`Session`](session::Session), the connection's identity and
//! address, and the capabilities above, and every scope the client
//! opens is read, routed and served in a task of its own until the
//! connection ends. One call per connection is the whole of a
//! provider's loop.

pub mod authorization;
pub mod channel;
pub mod container;
pub mod container_deployer;
pub mod deployment;
pub mod handle;
pub mod image_checker;
pub mod mount;
mod notice;
pub mod received;
pub mod scope_handle;
pub mod session;
pub mod unbrokered_authorizer;
pub mod volume_manager;
