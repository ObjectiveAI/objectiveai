//! Whether a provider can supply an image, and, where it can,
//! everything read of it: the run's OCI asks the other way round, and
//! the image's own document.
//!
//! The scope's request is the question: an
//! [`Image`](crate::shared::containers::request::Image) — its digest,
//! and the references under which its bytes may be fetched — the
//! same value a run names its image by and the same request the
//! provider sends a caller in
//! [`oci::has`](crate::shared::containers::oci::has), so an image
//! asked about here is one a run can ask for, with nothing to
//! translate between them. The first response answers it. Where the
//! answer is held, the scope stays open, and the caller asks on
//! channels what a run's provider asks of its caller: a manifest by
//! digest, a blob by digest, in pieces, and the proof over the image.
//! The request bodies and the answers are those of
//! [`oci`](crate::shared::containers::oci); only who asks has changed.
//! One ask more is the scope's own: the image's `/DIVERGE.md`, which
//! the provider reads out of the held image without running it and
//! answers parsed, as [`info`](crate::shared::containers::info) states
//! it — the document, that the image ships none, or an error. It is
//! the one place a document is asked for: no container scope answers
//! it, and nothing is read of an image the scope does not hold.
//!
//! # The rules
//!
//! 1. **The caller's Store first.** The provider answers
//!    [`Held`](server::response::Frame::Held) when the caller owns
//!    the image in its Store on this provider — one it
//!    [built](super::build) or [pushed](super::push) and has not
//!    deleted.
//! 2. **Then a registry it is configured with.** Failing that, it
//!    answers `Held` when a registry it is configured with — the
//!    `images_registries` of its [`info`](crate::provider::endpoints::info)
//!    — named by one of the image's references, serves the image's
//!    digest. A reference naming a registry it is not configured with
//!    is ignored.
//! 3. **Never another's.** It never answers from another identity's
//!    Store, nor from images it holds only because other clients
//!    supplied them to runs: a read scope reveals and serves nothing a
//!    caller could not obtain for itself.
//! 4. **Otherwise not found.** An image it cannot supply from either
//!    is [`NotFound`](server::response::Frame::NotFound), and says
//!    nothing about why — "I do not have it" and "I will not serve it
//!    to you" are the same answer from where the caller stands.
//! 5. **Unsupported.** A provider with neither a Store — its
//!    `images_store` is `false` — nor any registry — its
//!    `images_registries` is empty — answers
//!    [`Unsupported`](server::response::Frame::Unsupported), and only
//!    such a provider does.
//! 6. **Exactly one answer, first.** Exactly one of `Held`,
//!    `NotFound`, `Unsupported` and
//!    [`Error`](server::response::Frame::Error) is the first response
//!    on the main stream. After `NotFound`, `Unsupported` or `Error`
//!    the finish follows at once, and no channel is answered.
//! 7. **One image, served from where it was found.** A held scope
//!    serves the manifests, the blobs and the proof of the one image
//!    its request names, and of no other — from the caller's Store, or
//!    else from a configured registry one of the Image's references
//!    names, as rules 1 to 3 found it.
//! 8. **Only what the image reaches.** `manifest` and `blob` are
//!    answered only for digests the image reaches: its manifest, every
//!    manifest an index lists, and the config and layers of each of
//!    those. A digest outside that reach gets the empty finish.
//! 9. **The proof over the scope's image.**
//!    [`OciProve`](client::channel_request::Frame::OciProve) is
//!    answered with `SHA-256(nonce ‖ D)` over the scope's image, as
//!    [`oci::prove`](crate::shared::containers::oci::prove) defines
//!    `D`, under the ask's platform and nonce; a platform that breaks
//!    that module's rule gets the empty finish. It moves no byte of
//!    the image.
//! 10. **The caller ends it.** After `Held`, nothing more is sent on
//!     the main stream until the finish, and the finish follows the
//!     caller's [`Cancel`](client::channel_request::Frame::Cancel) — a
//!     channel nothing answers — the caller gone, or the caller
//!     reading no more.
//!
//! # A held is not a promise
//!
//! The answer is true when it is given. A registry may stop serving
//! the image, and the caller may delete it from its Store, while the
//! scope is open; an ask the provider can no longer answer gets the
//! empty finish, as one outside the image's reach does.
//!
//! # What it is for
//!
//! A caller that holds images on several providers, or can name one a
//! provider pulls from a registry, can answer one provider's asks from
//! another: asked on a run scope for the run's image, it finds a
//! provider that answers a read scope held, and relays the manifests,
//! the blobs and the proof from that one scope. The caller needs no
//! store of its own, and the provider pulling never learns where the
//! bytes came from.
//!
//! Split by who SENDS, as everywhere else: the request, the channels
//! and the executor are in [`client`]; the answers in [`server`].

pub mod client;
pub mod server;
