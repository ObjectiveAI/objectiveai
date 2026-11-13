//! Building an image on a provider, from a Containerfile. One
//! request, a stream of answers.
//!
//! A client names a provider, hands the Containerfile, a description
//! if it has one, and — if the Containerfile copies from one — names a
//! directory of a volume on that provider as the build's context. The
//! daemon asks the provider's
//! [`images::build`](crate::provider::endpoints::images::build) as
//! itself, and relays what the provider answers: every line the build
//! says, as it says it, and then the image built — its digest and its
//! size — or that it was owned already, or one of the refusals, or a
//! failure; then the scope finishes.
//!
//! The image is built into the DAEMON's Store on that provider: the
//! daemon is the provider's caller, so what it builds it owns there,
//! and every container the daemon runs on that provider naming the
//! digest runs it from the Store, with nothing pulled. Nothing of the
//! build is a record of the daemon's; the digest is the whole of what
//! a client takes away, and it is what a template names.
//!
//! Two grants. `build` over images, a bare action, since an image is
//! no record to narrow by. And, where a context is named, `mount` over
//! that volume: a build takes the hold on a volume a container's mount
//! takes, and reads what a container mounting it could read.
//!
//! A build cannot be stopped from the client's side: the provider
//! gives its caller no way to, and a client that goes away leaves the
//! build to finish and the image, if it is built, in the Store.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answers
//! are in [`server`].

pub mod client;
pub mod server;
