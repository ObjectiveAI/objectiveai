//! Pulling one image's manifests and blobs: two of the run's OCI
//! asks, the other way round.
//!
//! On a run scope the provider asks the caller for a manifest and for
//! a blob, by digest, to take the run's image from it. Here the caller
//! asks the provider the same two things, as channels on a scope it
//! opens for ONE image — the
//! [`Image`](crate::shared::containers::request::Image) its request
//! names — and keeps until it cancels: a manifest by digest, and a
//! blob by digest, in pieces. The request bodies and the answers are
//! those of [`oci`](crate::shared::containers::oci); only who asks has
//! changed.
//!
//! # The rules
//!
//! 1. **One image.** The scope serves the manifests and blobs of the
//!    one image its request names, and of no other.
//! 2. **From where the provider can supply it.** The image is served
//!    from the caller's Store, or from a registry the provider is
//!    configured with that one of the Image's references names —
//!    never from another identity's Store, nor from images the
//!    provider holds only because other clients supplied them to runs.
//! 3. **Only what the image reaches.** `manifest` and `blob` are
//!    answered only for digests the image reaches: its manifest, every
//!    manifest an index lists, and the config and layers of each of
//!    those. A digest outside that reach gets the empty finish.
//! 4. **No proof here.**
//!    [`prove`](crate::shared::containers::oci::prove) is not on this
//!    scope: the proof over an image is [`hash`](super::hash)'s, which
//!    moves no byte.
//! 5. **The caller ends it.** The scope lasts until the caller's
//!    [`Cancel`](client::channel_request::Frame::Cancel) — a third
//!    channel, which nothing answers — or the caller's connection
//!    ending; the provider's response finish follows either.
//!
//! # Nothing on the main stream
//!
//! The provider answers nothing on channel `0` of a scope it serves:
//! every ask is answered on its own channel, and the finish is the
//! scope's end. A finish the caller did not ask for is a scope the
//! provider will not serve. The exceptions are the two frames of
//! [`server::response`], each sent first and alone, then the finish:
//! [`NotFound`](server::response::Frame::NotFound), the provider
//! cannot supply the image, and
//! [`Unsupported`](server::response::Frame::Unsupported), it has
//! neither a Store — its `images_store` is `false` — nor any registry
//! — its `images_registries` is empty; see [`info`](super::super::info).
//!
//! # What it is for
//!
//! A caller that holds images on several providers, or can name one a
//! provider pulls from a registry, can answer one provider's asks from
//! another: asked on a run scope for the run's image, it finds a
//! provider that answers [`has`](super::has) held, opens a pull of the
//! image there, and relays the manifests and blobs from it, and the
//! proof from that provider's [`hash`](super::hash). The caller needs
//! no store of its own, and the provider pulling never learns where
//! the bytes came from.
//!
//! Split by who SENDS, as everywhere else: the request, the channels
//! and the executor are in [`client`]; the answers in [`server`].

pub mod client;
pub mod server;
