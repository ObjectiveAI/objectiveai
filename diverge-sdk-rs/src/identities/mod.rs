//! The host's OpenID Connect identities: one file each, signed in,
//! kept fresh.
//!
//! An identity is one subject at one issuer, signed in once and then
//! kept signed in by its refresh token. Every Diverge program on a
//! host shares the identities under its root:
//!
//! ```text
//! <root>/identities/
//!   https%3A%2F%2Fauth.example.com/   the issuer, percent-encoded
//!     alice.json                      the identity, by subject
//!     alice.lock                      its lock
//! ```
//!
//! The rules:
//!
//! 1. **Names.** The issuer's folder and the subject's file are each
//!    the name percent-encoded: every byte but `A–Z a–z 0–9 - . _ ~`
//!    as `%XX`, and the first character too where the name would
//!    otherwise be one Windows reserves, or a trailing `.`, which
//!    Windows drops. The issuer and the subject
//!    are written inside the file as well, and the file is the word on
//!    them.
//! 2. **Owner-only.** The folders are made `0700` and the files
//!    `0600` on Unix; on Windows they take the user profile's ACL. A
//!    file holds a refresh token, which is a sign-in.
//! 3. **Turns.** Every reader and writer of an identity takes its
//!    `.lock` first, with [`file_lock::wait_exclusive`](crate::file_lock::wait_exclusive),
//!    across processes and across tasks alike, and lets it go last. A
//!    file is written whole to a staging file beside it and renamed
//!    over it, so a file is never half written.
//! 4. **A token, fresh.** [`access_token`](Identities::access_token)
//!    and [`id_token`](Identities::id_token) take the lock, read the
//!    file, and answer the token if it is good for longer than the
//!    `valid_for` asked. Else they refresh — the refresh token
//!    presented, the newest refresh token answered kept, the old one
//!    where none is — write the file, let the lock go, and answer the
//!    new token. A new token good for no longer than `valid_for` is
//!    [`Error::TooShort`]: no caller is handed a token that fails the
//!    bound it set.
//! 5. **Signed out.** A refresh the issuer refuses as `invalid_grant`
//!    — the token rotated away, revoked, or expired — is
//!    [`Error::SignedOut`]. The file is kept, as user state is; a new
//!    login is what replaces it. Any other refusal is
//!    [`Error::Refused`], with what the issuer said.
//! 6. **Login.** [`login`](Identities::login) is OAuth's device
//!    authorization grant, RFC 8628, as a public client: the issuer's
//!    discovery document read, and it must name the issuer exactly as
//!    given, which must be `https`, or `http` on a loopback host; a
//!    device code asked for; and [`Pending::wait`] polling the token
//!    endpoint at the interval, five seconds slower at each
//!    `slow_down`, until a person approves the code — the identity
//!    written, replacing one already there — or denies it
//!    ([`Error::Denied`]), or it expires ([`Error::Expired`]). The
//!    scope asked for gains `openid` and `offline_access` where it
//!    lacks them: the ID token names the subject, and `offline_access`
//!    is how most issuers are asked for a refresh token.
//! 7. **The subject** is the ID token's `sub`, read without the
//!    signature checked: OpenID Connect Core §3.1.3.7 allows it for an
//!    ID token taken from the token endpoint itself, over TLS. An
//!    answer with no ID token, or one with no `sub`, is
//!    [`Error::Subject`].
//! 8. **The audience.** The access token is for what the login asked
//!    for as `resource`, RFC 8707's, or the issuer's own choice where
//!    it asked for none; a `resource` is asked again at every refresh,
//!    so the token stays for it.
//!
//! [`Token`] is a token handed out: it has no way out but
//! [`expose`](Token::expose), and its memory is zeroed on drop. The
//! file's own record has no public type at all.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod claims;
mod encode;
mod error;
mod fresh;
mod identities;
mod list;
mod login;
mod name;
mod oauth;
mod record;
mod refresh;
mod remove;
mod token;

pub use error::*;
pub use identities::*;
pub use login::*;
pub use name::*;
pub use token::*;
