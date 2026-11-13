//! What the tool asks of the daemon, declared at its registration.

use diverge_sdk::shared::permission::identities::{Make, Over, Permission};
use diverge_sdk::shared::permission::{Grant, Within};

/// The tool's permissions: to log in, and nothing else. The rules:
///
/// 1. **The login that makes.** `{"identities": ["login"]}`: signing
///    in an identity that is not on the daemon's host yet — a subject's
///    first login there.
/// 2. **The login that replaces.** `{"identities": {"actions":
///    ["login"], "within": "any"}}`: signing in again one that is,
///    whatever its tags — a subject signed out, or signed in anew.
/// 3. **No more.** The tool lists no identity and tags none: what is
///    on the host is not its to read, and it asks the daemon for
///    nothing but the login.
///
/// Both are what `/DIVERGE.md` says the tool needs. A daemon that
/// grants the tool neither answers every login `forbidden`.
pub fn permissions() -> Vec<Grant> {
    vec![
        // Rule 1.
        Grant::Identities(Permission::Make(vec![Make::Login])),
        // Rule 2.
        Grant::Identities(Permission::Over { actions: vec![Over::Login], within: Within::Any }),
    ]
}
