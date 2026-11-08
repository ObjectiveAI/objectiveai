//! Whether a provider can supply an image.
//!
//! One question, one answer. The question is an
//! [`Image`](crate::shared::containers::request::Image) — its digest,
//! and the references under which its bytes may be fetched — the
//! same value a run names its image by and the same request the
//! provider sends a caller in
//! [`oci::has`](crate::shared::containers::oci::has), so an image
//! asked about here is one a run can ask for, with nothing to
//! translate between them.
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
//!    supplied them to runs: a `has` reveals nothing a caller could
//!    not obtain for itself.
//! 4. **Otherwise not found.** An image it cannot supply from either
//!    is [`NotFound`](server::response::Frame::NotFound), and says
//!    nothing about why — "I do not have it" and "I will not serve it
//!    to you" are the same answer from where the caller stands.
//! 5. **Unsupported.** A provider with neither a Store nor any
//!    registry answers
//!    [`Unsupported`](server::response::Frame::Unsupported).
//! 6. **Exactly one answer, then the finish.**
//!
//! # A held is not a promise
//!
//! The answer is true when it is given. A registry may stop serving
//! the image, and the caller may delete it from its Store, before a
//! run asks for it; a run that follows a held looks again, as every
//! run does, and says where it found the image on its own answer.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and a provider answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
