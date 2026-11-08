//! Whether the caller possesses an image: the nonce check.
//!
//! The provider opens a channel with a [`request::Request`] — the
//! [`Image`](crate::shared::containers::request::Image), the platform
//! manifest's digest when the image's manifest is an index, and a
//! fresh nonce — and the caller
//! answers with one [`response::Frame`], the 32 bytes of the proof,
//! then the finish; the empty finish is not held. It is the last ask
//! of every image a provider takes from the caller, by a run or by an
//! [`images::push`](crate::provider::endpoints::images::push),
//! whether or not the provider pulled any byte of it: a digest names
//! an image, and only the bytes prove that the caller has them.
//!
//! # The proof
//!
//! 1. **The nonce.** Exactly 32 bytes, which the provider draws from
//!    a cryptographic random source for this one check and uses for
//!    no other.
//! 2. **The image's bytes, `D`.** Defined over the image
//!    `image.digest` names, and over nothing the references say: a
//!    reference is only where the image's bytes may be fetched, and
//!    the same bytes under any name prove the same. The concatenation,
//!    in this order and with no separator between any two parts, of:
//!    1. the bytes of the manifest named by `image.digest`, verbatim;
//!    2. when that manifest is an image index: the bytes of the
//!       manifest named by `platform`, verbatim;
//!    3. the bytes of every blob that the image manifest references —
//!       the platform manifest when there is an index, the manifest
//!       named by `image.digest` otherwise: the config first, then each
//!       entry of its `layers`, in order — an entry naming the same
//!       blob as an earlier one is hashed again.
//! 2a. **The platform, required or forbidden.** `platform` is
//!    present exactly when the manifest named by `image.digest` is an index,
//!    and then names a manifest that index lists. An ask that breaks
//!    this names no image: it is answered not held, and the check
//!    fails. No proof is ever computed over an index alone. Where the
//!    provider obtains an index, it selects the manifest for its own
//!    operating system and architecture, and every ask it sends names
//!    that one.
//! 3. **The proof.** `SHA-256(nonce ‖ D)`: the 32-byte digest of the
//!    nonce followed by `D`. [`proof::Prover`] computes it as the
//!    bytes arrive, and [`proof::proof`] over parts already in hand.
//! 4. **The comparison.** The provider computes the same value over
//!    its OWN copy of the image's bytes, and the check passes only
//!    when the two are equal, byte for byte. A different value, the
//!    empty finish, or an answer that is not one frame of exactly 32
//!    bytes fails it.
//! 5. **What a failure does.** The run or the push that asked fails —
//!    `ProofFailed` on its main stream — and the provider removes what
//!    that attempt brought in.
//!
//! # Why a nonce
//!
//! A digest is public: a caller that has seen one in a listing, a
//! registry or a log can name it without holding a byte of the image.
//! A hash keyed by a nonce the caller has never seen cannot be
//! answered from anything but the bytes, and it cannot be replayed:
//! the next check has another nonce.

pub mod proof;
pub mod request;
pub mod response;
