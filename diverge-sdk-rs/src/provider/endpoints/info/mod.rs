//! Asking a provider what it is: its version, and what it does.
//!
//! One question, one answer, and the smallest exchange in this
//! specification — shared only with [`capacity`](super::capacity),
//! shaped the same: a client asks, a provider says the revision of
//! the specification it implements and the capabilities it holds out,
//! and the scope finishes.
//!
//! # It is the one thing askable before anything else is known
//!
//! Every other scope assumes something. A [`run`](super::containers)
//! assumes the provider understands the request being sent; a
//! [`check`](super::images) assumes it agrees about what an image is.
//! This assumes only that the connection carries frames, which is the
//! least a peer can be doing and still be a peer.
//!
//! So it is what a client asks first when it does not yet know what it
//! is talking to, and the one exchange whose answer can be acted on
//! before any other request is composed: the version says which
//! specification the far end speaks, and the
//! [`Capabilities`](server::response::Capabilities) say which of its
//! requests the far end serves, so that a caller composes nothing the
//! answer has already refused.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and a provider answers, so the answer
//! is in [`server`].

pub mod client;
pub mod server;
