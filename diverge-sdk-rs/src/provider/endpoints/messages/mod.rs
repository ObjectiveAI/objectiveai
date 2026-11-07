//! Direct messages between the identities connected to a provider.
//!
//! Two scopes, each held at most once per connection — see
//! [the endpoints](super#one-open-at-a-time). [`listen`] is a stream
//! kept open: every message sent to the caller's identity, one
//! response each, until the caller cancels. [`send`] is one message to
//! one identity, and its one answer: delivered — the receiver's reader
//! took it — not listening, or rate limited. The rules, which both
//! scopes' docs refine:
//!
//! 1. **Direct only.** A message names exactly one receiver, by its
//!    identity, and reaches only that identity's listen scope on this
//!    provider. There is no broadcast and no store: a receiver with no
//!    listen scope open is not listening, and nothing is kept for it.
//! 2. **Received is taken.** A send is answered delivered only once
//!    the receiver's reader has taken the message off its queue: the
//!    listen scope's room coming back, see
//!    [`credit`](crate::wire::credit), is the receipt.
//! 3. **One in flight.** A connection has at most one send open, so a
//!    provider holds at most one message per sending connection,
//!    however slowly its receivers read.
//! 4. **The rate is required.** A provider that relays messages states
//!    a fixed-window rate in its
//!    [capabilities](crate::provider::endpoints::info::server::response::MessagesCapabilities);
//!    a provider that states none answers both scopes `Unsupported`.
//!
//! Split by who SENDS, as everywhere else.

pub mod listen;
pub mod send;
