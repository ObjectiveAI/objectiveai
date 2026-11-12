//! The host's accounts at `identity.diverge.network`: one file each,
//! signed in at the issuer of its own tokens, and what each may ask
//! of `identity.diverge.network` as itself.
//!
//! `identity.diverge.network` is Diverge's own OpenID Connect
//! provider. Every subject there is `<issuer>.<id>.<name>`: who
//! issued the owner's own sign-in — `github`, `google` — the id of a
//! user of it, and a name of that user's choosing. An **account** is
//! that user — `<issuer>.<id>`, the first two parts of every subject
//! it owns — and what proves it to `identity.diverge.network` is its
//! issuer's own token, which this module signs in for and keeps:
//!
//! ```text
//! <root>/identity.diverge.network/
//!   github.12345.json        the account, by issuer and id
//!   github.12345.lock        its lock
//! ```
//!
//! "Issuer" here is always the account's — GitHub, Google — and
//! never `identity.diverge.network` itself, which is the issuer of
//! the subjects' tokens and is called by its name.
//!
//! The rules:
//!
//! 1. **The issuers are a table** — see [`ISSUERS`]. An issuer is
//!    named by a word, and everything this module does for it is read
//!    off its entry: where a device code is asked for, where it is
//!    spent, what is asked, how the user's id is learned, and which
//!    of its tokens `identity.diverge.network` takes. One more issuer
//!    is one more entry.
//! 2. **Signed in by a device code.** [`login`](Accounts::login) is
//!    OAuth's device authorization grant, RFC 8628, at the issuer, as
//!    the client `identity.diverge.network` is there: a device code
//!    asked for, and [`Pending::wait`] polling until a person approves
//!    the user code at the issuer's own page, or denies it
//!    ([`Error::Denied`]), or it expires ([`Error::Expired`]).
//! 3. **Whose it is, learned after.** A login names an issuer and no
//!    user: who signed in is what the issuer says of the tokens it
//!    answered — see [`Who`] — so an account's file is written once
//!    the login ends, and is named by what it found.
//! 4. **Replacing.** A login as a user already signed in replaces that
//!    account's file: its tokens are the new ones.
//! 5. **Owner-only.** The folder is made `0700` and the files `0600` on
//!    Unix; on Windows they take the user profile's ACL. A file holds
//!    the issuer's tokens, which are the account. A file is written
//!    whole to a staging file beside it and renamed over it, so one is
//!    never half written.
//! 6. **Turns.** Every reader and writer of an account's tokens takes
//!    its `.lock` first, with
//!    [`file_lock::wait_exclusive`](crate::file_lock::wait_exclusive),
//!    across processes and across tasks alike — as an identity's is
//!    taken, see [`identities`](crate::identities), rule 3 — and lets
//!    it go as soon as it has a token good to use, before the request
//!    the token is for. So of two that find a token expired, one
//!    renews it and the other reads what the first wrote.
//!    [`list`](Accounts::list) reads names and no file, and takes no
//!    lock.
//! 7. **A token, good now** — see [`token`](Accounts::token), the one
//!    way a token is had. One that has expired — Google's, in an hour
//!    — is renewed by the account's refresh token, and the file
//!    rewritten. An account its issuer no longer answers for is
//!    [`Error::SignedOut`]: its file is kept, and a new login replaces
//!    it.
//! 8. **No token is answered** by anything public here:
//!    [`list`](Accounts::list) reads names, [`info`](Accounts::info)
//!    answers what the issuer says of the user, and the requests of
//!    `identity.diverge.network` — see [`service`](mod@service) —
//!    answer what it does, none of which holds one.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod account;
mod accounts;
mod device;
mod error;
mod info;
mod issuer;
mod list;
mod login;
mod pending;
mod record;
mod service;
mod token;
mod who;

pub use account::*;
pub use accounts::*;
pub use error::*;
pub use issuer::*;
pub use pending::*;
pub use service::*;
