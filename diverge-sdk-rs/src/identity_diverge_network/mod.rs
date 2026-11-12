//! The host's accounts at `identity.diverge.network`: one file each,
//! signed in at an external provider.
//!
//! `identity.diverge.network` is Diverge's own OpenID Connect
//! provider. Every subject there is `<provider>.<id>.<name>`: an
//! external provider, the id of a user of it, and a name of that
//! user's choosing. An **account** is that user — `<provider>.<id>`,
//! the first two parts of every subject it owns — and what proves it
//! to `identity.diverge.network` is the external provider's own
//! token, which this module signs in for and keeps:
//!
//! ```text
//! <root>/identity.diverge.network/
//!   github.12345.json        the account, by provider and id
//! ```
//!
//! The rules:
//!
//! 1. **The providers are a table** — see [`PROVIDERS`]. A provider is
//!    named by a word, and everything this module does for it is read
//!    off its entry: where a device code is asked for, where it is
//!    spent, what is asked, and how the user's id is learned. One more
//!    provider is one more entry.
//! 2. **Signed in by a device code.** [`login`](Accounts::login) is
//!    OAuth's device authorization grant, RFC 8628, at the external
//!    provider, as the client `identity.diverge.network` is there: a
//!    device code asked for, and [`Pending::wait`] polling until a
//!    person approves the user code at the provider's own page, or
//!    denies it ([`Error::Denied`]), or it expires
//!    ([`Error::Expired`]).
//! 3. **Whose it is, learned after.** A login names a provider and no
//!    user: who signed in is what the provider says of the tokens it
//!    answered — see [`Who`] — so an account's file is written once
//!    the login ends, and is named by what it found.
//! 4. **Replacing.** A login as a user already signed in replaces that
//!    account's file: its tokens are the new ones.
//! 5. **Owner-only.** The folder is made `0700` and the files `0600` on
//!    Unix; on Windows they take the user profile's ACL. A file holds
//!    the external provider's tokens, which are the account. A file is
//!    written whole to a staging file beside it and renamed over it, so
//!    one is never half written.
//! 6. **No token is answered** by anything here but to this module's
//!    own later use: [`list`](Accounts::list) reads names, and no
//!    token.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod account;
mod accounts;
mod device;
mod error;
mod list;
mod login;
mod pending;
mod provider;
mod record;
mod who;

pub use account::*;
pub use accounts::*;
pub use error::*;
pub use pending::*;
pub use provider::*;
