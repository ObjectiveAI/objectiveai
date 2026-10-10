//! Asking a runner whether a connector may join.
//!
//! A [`Connect`](crate::shared::containers::request::Connect) arrives
//! at the provider carrying an authorization the provider cannot
//! judge, so the provider asks whoever runs the container: it opens a
//! channel on the run scope with [`request::AuthorizeConnect`] — where the
//! connector's socket came from, and what it offered — and the runner
//! answers with one [`response::Frame`], yes or no, then the channel
//! finishes. The connect scope opens on a yes and is refused on a no.
//!
//! The ask is the tools family's: an agent container takes no
//! connector, and is never asked. Once the connector is attached the
//! runner is told so on its run scope's main stream, and told again
//! when the connector leaves: see the tools run's
//! [`response`](crate::provider::endpoints::containers::tools::run::server::response).
//!
//! This layer guarantees two things and no more: that the bytes
//! arrive as they were sent, and that the question is answered before
//! the connection it is about is allowed to open.

pub mod request;
pub mod response;
