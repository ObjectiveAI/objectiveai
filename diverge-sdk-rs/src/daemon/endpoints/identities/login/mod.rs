//! Signing an identity in on the daemon's host. A client names the
//! identity and the public client to sign in as. The daemon begins the
//! device authorization grant, RFC 8628 — the one login there is — and
//! answers what is needed to approve it — the user code, and where it
//! is approved — then, once it is decided or the code expires, the
//! outcome. Then the scope finishes.
//!
//! The rules:
//!
//! 1. **On the daemon's host.** The daemon asks the issuer, polls it,
//!    and writes the identity among its host's own: the client never
//!    talks to the issuer, and no token reaches the client.
//! 2. **Approved anywhere.** A device login's code is approved in any
//!    browser that reaches the issuer's verification page — the
//!    client's, or another device's — or by a request, where the
//!    issuer takes approvals so.
//! 3. **Replacing.** An approval as the identity named writes it,
//!    replacing one already there, signed out or not; an approval as
//!    another identity writes nothing.
//! 4. **Never waiting for the lock.** The identity's lock is taken only
//!    if nobody holds it: another login of it, or a reader, is
//!    [`Held`](server::response::Frame::Held) at once, and nothing is
//!    asked of the issuer.
//! 5. **A cancel** — or the client's connection ending — ends the
//!    login, and nothing is written.
//! 6. **Not presented.** The identity is the host's from then on; which
//!    subject the daemon presents at an issuer is still its
//!    configuration's.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
