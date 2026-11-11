//! Reading an image's DIVERGE.md. One request, one answer.
//!
//! A client names an image, and the daemon answers its `/DIVERGE.md`:
//! from a container of that image it runs, when it runs any, through
//! the run's `info` channel; else from a provider that can obtain the
//! image, which reads the file out of the image without running it
//! ([`containers::info`](crate::provider::endpoints::containers::info)).
//! The answer is the document, that the image ships none, that nothing
//! of the image runs and no provider can obtain it, forbidden, or a
//! failure, and the scope finishes. Nothing is retried.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
