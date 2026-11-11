//! Reading an image's DIVERGE.md, without running it.
//!
//! One question, one answer: a client names an image, and the provider
//! answers the image's `/DIVERGE.md` — the JSON Schema of the arguments
//! its program accepts, and the prose beneath — that the image ships
//! none, or that it can find no copy of the image. Nothing runs: the
//! provider reads the file out of the image, where the `info` channel
//! of a [run](super::agents::run) asks the running container for it.
//! What it answers is the same
//! [`Document`](crate::shared::containers::info::Document) either way.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and a provider answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
