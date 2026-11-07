//! Registering a tool another daemon holds, under a name.
//!
//! One request, one answer. A client names a daemon it holds an
//! account on — a record of
//! [`daemons`](crate::daemon::endpoints::daemons)
//! — and a tool as that daemon names it, and the name it wants the
//! tool held under; the daemon answers that the tool exists under the
//! name, that the name is already in use, or that it failed, and the
//! scope finishes. Nothing is connected at the request: while an
//! agent the tool is attached to uses it, the daemon connects to the
//! daemon named through a provider both are connected to and opens
//! that daemon's [`connect`](super::connect) naming the tool, over
//! which the tool's MCP exchanges travel, and lets the connection go
//! when nothing has used the tool for `idle_seconds`. The tool's life
//! is not this scope's: it goes on after the finish, reached by its
//! name, attached to agents by an
//! [`attach_agent_instance`](super::attach_agent_instance) or an
//! [`attach_agent_template`](super::attach_agent_template), until a
//! [`delete`](super::delete).
//!
//! # Reached through other daemons
//!
//! A tool need not be on the daemon named: the register's `route`
//! names further remote-daemon records, and the tool's connect is
//! relayed along them, as any request's
//! [`relay`](crate::shared::relay::Relay) is. The rules:
//!
//! 1. **An explicit route.** `daemon` is a record of this daemon's, by
//!    its ID, and each ID of `route` a record of the daemon before it;
//!    the tool is on the last. An empty route is the tool on `daemon`.
//! 2. **Every hop is an account.** Each daemon along the route serves
//!    the connect as the account the previous daemon's record names,
//!    and judges that account's `relay` grant over its next record; the
//!    tool's daemon serves the connect itself as the last relaying
//!    daemon's account, and judges that account's grants over the tool
//!    — as a relayed request of any kind is served.
//! 3. **No limit.** A route is as long as the register writes it. It
//!    cannot loop: each hop consumes one ID.
//! 4. **Fixed at the register.** A connected tool is not edited; a
//!    tool reached by another route is another register.
//! 5. **Each hop is what the register meant.** An ID is the hash of
//!    what its record reaches and as whom — see
//!    [`DaemonId`](crate::daemon::endpoints::daemons::DaemonId) — so no
//!    daemon along the route can re-point a hop: it holds the record
//!    the ID names, or none, and the connect is refused there.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
