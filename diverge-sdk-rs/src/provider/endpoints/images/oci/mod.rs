//! Pulling an image in the caller's Store: three of the run's OCI
//! asks, the other way round.
//!
//! On a run scope the provider asks the caller — whether it holds an
//! image, for a manifest, for a blob — to take the run's image from
//! it. Here the caller asks the provider the same three things, as
//! channels on a scope it opens and keeps: whether the provider holds
//! the image in THE CALLER'S Store — one the caller built or pushed
//! there, and never another identity's nor one the provider merely
//! pulled; a manifest by digest; a blob by digest, in pieces. The
//! asks are byte for byte the run's, the same tags and the same
//! frames of [`oci`](crate::shared::containers::oci); only who asks
//! has changed.
//!
//! # The rules
//!
//! 1. **From the Store.** Every ask is answered from the caller's own
//!    Store, and from nothing else.
//! 2. **`has` answers a fixed label.** An image the caller owns is
//!    answered held, with the name `image`: a label, which the
//!    provider asking the caller puts in one pull's reference and
//!    never stores.
//! 3. **No proof here.** The fourth ask,
//!    [`prove`](crate::shared::containers::oci::prove), is not on
//!    this scope: the proof over a Store image is
//!    [`hash`](super::hash)'s, which moves no byte.
//!
//! # Nothing on the main stream
//!
//! The provider answers nothing on channel `0`, with one exception.
//! The scope is held for the connection's life, every ask answered on
//! its own channel, and it ends at the caller's disconnect — a fourth
//! channel, which nothing answers — or at the caller's connection
//! ending; the provider's response finish follows either. A finish
//! the caller did not ask for is a scope the provider will not serve.
//! The exception is a provider with no Store — its
//! `images_store` is `false`, see [`info`](super::super::info) — which says
//! so with the one frame of [`server::response`], first and alone,
//! and finishes.
//!
//! # What it is for
//!
//! A caller that holds images on several providers can answer one
//! provider's asks from another's: asked on a run scope whether it
//! holds an image, it finds a provider that does and relays the
//! manifests and blobs from there, and the proof from that provider's
//! [`hash`](super::hash). The caller needs no store of its own, and
//! the provider pulling never learns where the bytes came from.
//!
//! Split by who SENDS, as everywhere else: the request, the channels
//! and the executor are in [`client`]; the answers in [`server`].

pub mod client;
pub mod server;
