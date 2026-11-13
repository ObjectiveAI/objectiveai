//! Reading an image's DIVERGE.md. One request, one answer.
//!
//! A client names an image, and the daemon answers its `/DIVERGE.md`:
//! from its own record of the digest, where it has read the document
//! before; else from a container of that image it runs, when it runs
//! any, through the run's `info` channel; else from whoever holds the
//! image. Every connected provider and every remote daemon is asked
//! at once whether it holds it, as an
//! [`images::oci`](crate::daemon::endpoints::images::oci) scope asks,
//! and one that does is asked for the document: a provider reads the
//! file out of the image without running it
//! ([`containers::info`](crate::provider::endpoints::containers::info)),
//! and a remote daemon answers this same request, which the account
//! this daemon holds there needs the `info` grant over images for.
//! With no holder, every connected provider is asked for the document
//! outright, which finds an image only a provider's cache holds.
//!
//! A remote daemon asked searches in turn, so the request carries the
//! daemons not to ask — `ignore` — and each daemon adds itself before
//! it asks another: no daemon is asked twice along one path.
//!
//! The answer is the document, that the image ships none, that nothing
//! of the image runs and nobody in reach holds it, forbidden, or a
//! failure, and the scope finishes. A document found is put on the
//! daemon's record, and the next ask is answered from there.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
