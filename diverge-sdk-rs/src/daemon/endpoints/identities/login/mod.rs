//! Signing an identity in on the daemon's host. A client names the
//! identity and the public client to sign in as. With the identity's
//! password, the daemon asks the issuer once, OAuth's password grant,
//! RFC 6749 §4.3, and answers the outcome. Without one, it begins the
//! device authorization grant, RFC 8628, and answers what a person needs
//! to approve it — the user code, and where to enter it — then, once
//! the person decides or the code expires, the outcome. Then the scope
//! finishes.
//!
//! The rules:
//!
//! 1. **On the daemon's host.** The daemon asks the issuer, polls it,
//!    and writes the identity among its host's own: the client never
//!    talks to the issuer, and no token reaches the client.
//! 2. **Approved anywhere.** A device login's code is approved in any
//!    browser that reaches the issuer's verification page — the
//!    client's, or another device's. A password login has no code.
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
