//! The frame-level client: the half that mints scopes and channels.
//!
//! What a caller of the [`provider`](crate::provider) protocol holds,
//! and what a provider's server holds toward every container's proxy
//! — the [`outside`](crate::container_proxy::outside) wire it dials
//! into a container, where it mints the scopes and the proxy answers.
//! Same frame, same machinery, one implementation. The caller's own
//! answerers — what a provider asks a caller for while a container
//! runs — are the provider protocol's, in
//! [`provider::client`](crate::provider::client); nothing here knows
//! an endpoint.
//!
//! # What is here
//!
//! One socket: its [`handshake`](crate::wire::handshake) first, then
//! split in two. The handshake is the connection's, not this half's —
//! either end may be the connector — and the
//! [`Connection`](crate::wire::connection::Connection) it hands back is
//! what is split.
//!
//! [`router`] is the read loop: frames off the socket, forwarded to
//! whoever is waiting. [`handle`] is the write half and what a caller
//! keeps hold of. They are separate because a
//! [`Sink`](futures_util::Sink) needs `&mut` and a read loop never
//! finishes, so one type holding both could only ever do one of them.
//!
//! Two channels run between them. [`registration`] goes forward,
//! saying where frames should go before the request that causes them;
//! closures come back, saying an entry is gone.
//!
//! [`dial`] is the whole of it for a daemon: the socket, the
//! handshake as its connector, the split, and the router's run handed
//! back for the caller to spawn.
//!
//! [`scope`] and [`channel`] are what a [`handle`] hands back — the
//! two things a caller opens, each with the receivers its frames
//! arrive on. Those three are data and nothing else, which is why
//! they sit beside the handle and the router rather than inside
//! either: what makes one is the writer's business, what routes by
//! one is the reader's, and what to do with one is a caller's.
//!
//! # Every queue here is one frame ahead
//!
//! Each stream is paced by its reader (see [`credit`](crate::wire::credit)):
//! the far end sends a frame only into room this end's reader gave it
//! by taking the last. So a [`router`] filing a frame never waits on a
//! consumer — the room was there before the frame was sent — and one
//! scope that has stopped reading stalls its own sender, on the far
//! side, and nothing else. What any stream holds here is one frame and
//! its finish.
//!
//! The obligation on a caller is the same as ever: read what you
//! asked for, or drop it. Dropping tells the far end, whose sends on
//! that stream then cost nothing.

pub mod channel;
pub mod dial;
pub mod handle;
pub mod registration;
pub mod router;
pub mod scope;
