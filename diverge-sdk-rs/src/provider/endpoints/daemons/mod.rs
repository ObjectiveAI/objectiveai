//! Daemons reaching one another through the provider: connecting,
//! announcing, listing, and messaging.
//!
//! Six scopes. Two are one per end of a daemon connection. A daemon
//! that will take connections from other daemons holds an [`accept`] on
//! the provider; a daemon that wants one opens a [`connect`] naming the
//! acceptor by the identity the provider knows it by, and the provider
//! relays the daemon connection between the two for as long as both
//! scopes live — bytes it does not read, in the forms
//! [`shared::daemons`](crate::shared::daemons) states. Nothing of a
//! daemon's own wire is here: the connection carried is the daemon
//! protocol, whole, and the provider is the one thing both ends happen
//! to be connected to.
//!
//! [`announce`] is how an identity comes to be listed: a scope it
//! holds open, carrying its announcement, for as long as it wants to be
//! found. [`list`] lists the identities connected to the provider that
//! hold an announce open, each with its announcement, and keeps the
//! listing: what a daemon finds another by.
//!
//! Two more are direct messages between connected identities.
//! [`listen`] is a stream kept open: every message sent to the caller's
//! identity, one response each, until the caller cancels. [`message`]
//! is one message to one identity, and its one answer: delivered — the
//! receiver's reader took it — not listening, or rate limited. The
//! rules, which both scopes' docs refine:
//!
//! 1. **Direct only.** A message names exactly one receiver, by its
//!    identity, and reaches only that identity's listen scope on this
//!    provider. There is no broadcast and no store: a receiver with no
//!    listen scope open is not listening, and nothing is kept for it.
//! 2. **Received is taken.** A message is answered delivered only once
//!    the receiver's reader has taken it off its queue: the listen
//!    scope's room coming back, see [`credit`](crate::wire::credit), is
//!    the receipt.
//! 3. **One in flight.** A connection has at most one message open, so
//!    a provider holds at most one message per sending connection,
//!    however slowly its receivers read.
//! 4. **The rate is required.** A provider that relays messages states
//!    a fixed-window rate in its
//!    [capabilities](crate::provider::endpoints::info::server::response::MessagesCapabilities);
//!    a provider that states none answers both scopes `Unsupported`.
//!    The rate applies to every send.
//! 5. **Any connected identity.** Every connected identity may listen,
//!    and every connected identity may send, announced or not and
//!    listening or not. Neither scope requires the caller, or the
//!    receiver, to be listed.
//! 6. **Independent.** Announcing, accepting, listening and sending
//!    are independent of one another: an identity holds any of the
//!    four scopes without the others, and opening or ending one changes
//!    none of the others.
//!
//! [`list`], [`accept`], [`announce`], [`listen`] and [`message`] are
//! each held at most once per connection — see
//! [the endpoints](super#one-open-at-a-time).
//!
//! Split by who SENDS, as everywhere else.

pub mod accept;
pub mod announce;
pub mod connect;
pub mod list;
pub mod listen;
pub mod message;
