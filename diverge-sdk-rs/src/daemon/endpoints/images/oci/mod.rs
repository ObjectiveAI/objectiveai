//! Whether the daemon can supply an image, and, where it can, the OCI
//! asks of it on one scope: the provider's
//! [`images::oci`](crate::provider::endpoints::images::oci), asked of
//! the daemon.
//!
//! The scope's request is the question: an
//! [`Image`](crate::shared::containers::request::Image) — its digest,
//! and the references under which its bytes may be fetched — and the
//! daemons the daemon must not ask for it. The first response answers
//! it. Where the answer is held, the scope stays open, and the caller
//! asks on channels what a provider's oci scope answers: a manifest by
//! digest, a blob by digest, in pieces, and the proof over the image.
//! The channel requests and their answers are those of the provider's
//! scope, byte for byte.
//!
//! # The rules
//!
//! 1. **The daemon holds nothing.** It answers from a source it asks:
//!    on the request, it opens an oci scope with the image on every
//!    connected provider whose capabilities keep a Store or list a
//!    registry, and this scope on every remote daemon it records, all
//!    at once.
//! 2. **Only what the grants reach.** A provider is asked only where a
//!    `providers` grant reaches it with `oci`, and a remote daemon only
//!    where a `daemons` grant reaches its record with `oci`; see
//!    [`grant`](crate::daemon::grant). An account that holds `oci` over
//!    neither is answered
//!    [`Forbidden`](server::response::Frame::Forbidden), and nothing is
//!    asked.
//! 3. **Never one it is told to ignore.** A remote daemon whose
//!    identity is in the request's `ignore`, or is this daemon's own,
//!    is not asked. Asking a remote daemon, the daemon sends `ignore`
//!    with its own identity added, so a ring of daemons that record
//!    each other ends.
//! 4. **The first held answers.** The first source to answer held is
//!    the source: at that moment every other scope is cancelled, and
//!    the daemon answers [`Held`](server::response::Frame::Held).
//!    Every other answer from a source — not found, unsupported,
//!    forbidden, an error, a scope that would not open — is a no; with
//!    no source held, the answer is
//!    [`NotFound`](server::response::Frame::NotFound), after the last
//!    source has answered.
//! 5. **Exactly one answer, first.** Exactly one of `Held`, `NotFound`,
//!    `Forbidden` and [`Error`](server::response::Frame::Error) is the
//!    first response on the main stream. After any but `Held` the
//!    finish follows at once, and no channel is answered.
//! 6. **Served through the one source.** Every ask on a held scope is
//!    asked of the source on its own scope and answered as the source
//!    answered it. The daemon hashes nothing and keeps nothing; an ask
//!    the source fails is answered with the empty finish, and no other
//!    source is asked.
//! 7. **The caller ends it.** After `Held`, nothing more is sent on the
//!    main stream until the finish, and the finish follows the caller's
//!    [`Cancel`](client::channel_request::Frame::Cancel), the caller
//!    gone, or the source's scope ending. The source's scope is
//!    cancelled with it.
//!
//! Split by who SENDS, as everywhere else: the request, the channels
//! and the executor are in [`client`]; the answers in [`server`].

pub mod client;
pub mod server;
