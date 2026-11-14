//! Pushing an image into the daemon's Store on a provider, without
//! building it. One request, a stream of answers, and the channels the
//! daemon opens to ask for the image.
//!
//! The provider's
//! [`images::push`](crate::provider::endpoints::images::push), asked
//! of the daemon: a client names a provider and an image — its digest,
//! and the references under which its bytes may be fetched — and a
//! description if it has one, and the daemon asks that provider's push
//! as itself. The rules:
//!
//! 1. **Into the daemon's Store.** The daemon is the provider's
//!    caller, so what is pushed the daemon owns there, as it owns what
//!    an [`images::build`](super::build) builds: every container the
//!    daemon runs on that provider naming the digest runs it from the
//!    Store, and an [`images::list`](super::list) shows it.
//! 2. **Where the image comes from is the provider's order.** A digest
//!    the daemon owns there already is `Exists`, and nothing moves. A
//!    reference naming a registry the provider uses, which serves the
//!    digest, supplies it, and the client is asked nothing. Otherwise
//!    the CLIENT supplies it.
//! 3. **The provider's asks are the client's to answer.** Every
//!    channel the provider opens on its push — a manifest, each blob
//!    it does not hold, and, last and always, the proof — the daemon
//!    opens on the client's scope, the ask as the provider wrote it,
//!    and carries the client's answer back as the client wrote it. The
//!    daemon reads none of it, and supplies nothing of its own: it
//!    does not look to its other providers or to a remote daemon.
//! 4. **An ask the client does not answer is not held.** A client that
//!    reads no channel, that goes away, or that ends an answer short,
//!    is answered to the provider as the empty finish, which is what a
//!    caller that does not hold the digest says; the push is then
//!    `Unavailable`, or `ProofFailed`, and nothing is owned.
//! 5. **Judged by provider.** The caller holds `push` over images,
//!    reaching the provider named, by its identity or its tags: see
//!    [`grant::images`](crate::daemon::grant::images). Nothing else is
//!    judged: the image is the client's own to supply.
//! 6. **A push cannot be stopped** from the client's side: the client
//!    opens no channel on it.
//!
//! Split by who SENDS, as everywhere else: the request, the answers to
//! the daemon's asks and the executor are in [`client`]; the asks and
//! the answers in [`server`].

pub mod client;
pub mod server;
