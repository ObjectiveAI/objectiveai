//! Pulling an image a provider holds for the caller: the run's three
//! OCI asks, the other way round.
//!
//! On a run scope the provider asks the caller — whether it holds an
//! image, for a manifest, for a blob — to take the run's image from
//! it. Here the caller asks the provider the same three things, as
//! channels on a scope it opens and keeps: whether the provider holds
//! an image FOR THE CALLER — one the caller built on it, held byte
//! for byte as the provider fixed it at the build, and never another
//! identity's nor one the provider merely pulled, whose bytes it
//! cannot re-serve under the digest — and under what name; a manifest
//! by digest; a blob by digest, in pieces. The asks are byte for byte the run's, the same tags and
//! the same frames of [`oci`](crate::shared::containers::oci); only
//! who asks has changed.
//!
//! # Nothing on the main stream
//!
//! The provider answers nothing on channel `0`. The scope is held
//! for the connection's life, every ask answered on its own channel,
//! and it ends at the caller's disconnect — a fourth channel, which
//! nothing answers — or at the caller's connection ending; the
//! provider's response finish follows either. A finish the caller
//! did not ask for is a scope the provider will not serve.
//!
//! # What it is for
//!
//! A caller that holds images on several providers can answer one
//! provider's asks from another's: asked on a run scope whether it
//! holds an image, it finds a provider that does and relays the
//! manifests and blobs from there. The caller needs no store of its
//! own, and the provider pulling never learns where the bytes came
//! from.
//!
//! Split by who SENDS, as everywhere else: the request, the channels
//! and the executor are in [`client`]; the answers in [`server`].

pub mod client;
pub mod server;
