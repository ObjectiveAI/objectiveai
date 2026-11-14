//! Images: named by digest, never held.
//!
//! An image is no record of the daemon's. A request names one by its
//! [`Image`](crate::shared::containers::request::Image) — the digest
//! that is the image, and the references that say where its bytes may
//! be — and the daemon reaches it through what it does hold: a
//! provider, or a remote daemon, that can supply it.
//!
//! [`build`] makes one: a Containerfile, built on a provider the
//! client names, into the daemon's own Store there, so that every
//! container the daemon runs on that provider can name its digest.
//! Its grant is `{"images":["build"]}`, and `mount` over the volume
//! the build reads its context from, where it reads one.
//!
//! [`read`] is the image itself: whether the daemon can supply it, from
//! a provider or a remote daemon it asks, and, where it can, everything
//! read of it on one scope — its manifests, its blobs, its proof, and
//! its DIVERGE.md, which says what the arguments of a container of it
//! may be. It is reached by the `read` action of the `providers` and
//! `daemons` grants, over the sources it asks. Over images themselves a
//! grant names nothing, `{"images":["build"]}`: see
//! [`grant::images`](crate::daemon::grant::images).

pub mod build;
pub mod read;
