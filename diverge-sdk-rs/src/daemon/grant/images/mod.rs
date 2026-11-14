//! Grants over images: the daemon's Stores, provider by provider.
//!
//! An image is no record of the daemon's: a request names one by its
//! digest. What the daemon does hold is a Store on each provider that
//! keeps one — the images it built or was pushed there — and that is
//! what a grant over images reaches. On the wire an object of the
//! [`Action`]s held and how far they reach:
//!
//! ```json
//! {"images": {"actions": ["build", "push", "list"], "within": "any"}}
//! {"images": {"actions": ["build", "push"], "within": {"identities": [{"issuer": "…", "subject": "…"}]}}}
//! {"images": {"actions": ["list"], "within": {"all_tags": ["shared"]}}}
//! ```
//!
//! `within` is `"any"`, or a
//! [`Reach`](crate::daemon::grant::providers::Reach): the providers
//! named by identity, those whose records carry some tags, or both.
//! The rules:
//!
//! 1. **Every action is judged by the provider.** `build` and `push`
//!    by the provider the request names; `list` by the provider each
//!    image is on, so the list sends the Stores some grant reaches.
//! 2. **Nothing of the image is judged**: not its digest, not when it
//!    came to be owned. A build has no digest when it is judged, and a
//!    grant that reached some images of a Store and not others would
//!    narrow nothing a build could not undo.
//! 3. **The Store is the daemon's.** A grant narrows which providers'
//!    Stores an account may see and add to, never whose images: every
//!    account reaching a provider reaches the same Store there.
//! 4. **Reading is elsewhere.**
//!    [`images::read`](crate::daemon::endpoints::images::read) is held
//!    by `read` over providers and `relay` over daemons, and by no
//!    action here.
//!
//! [`Action`] is defined once in
//! [`shared::permission`](crate::shared::permission) and re-exported;
//! [`Permission`] is the object as a type.

mod permission;

pub use crate::shared::permission::images::Action;
pub use permission::*;
