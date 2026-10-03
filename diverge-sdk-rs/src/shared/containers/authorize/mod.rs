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
//! A second ask has the same answer. A
//! [`containers::tools::list_for`](crate::provider::endpoints::containers::tools::list_for)
//! names an identity, and the provider asks every tool container that
//! identity runs, on its run scope, with [`request::AuthorizeList`] —
//! who is asking, attested — whether the lister may see it; the
//! runner answers with the same one [`response::Frame`], and the
//! container is listed on a yes and not on a no. Both asks are the
//! tools family's: an agent container takes no connector and is
//! listed to nobody, and is asked neither.
//!
//! This layer guarantees two things and no more: that the bytes
//! arrive as they were sent, and that the question is answered before
//! the connection it is about is allowed to open, or the container
//! it is about is named.

pub mod request;
pub mod response;
