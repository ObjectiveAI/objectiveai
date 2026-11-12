//! The `identities` command, done.

use std::path::Path;

use diverge_sdk::identities::{self, Name};
use serde_json::json;

use super::Error;
use super::print::print;
use crate::arguments::Identities;
use crate::detach;
use crate::ours;

/// Do `command` to the identities at the root, printing one JSON line
/// per answer. The rules:
///
/// 0. **Login at `identity.diverge.network`** is its own way — see
///    [`ours`]: the subject's owner is an account signed in on this
///    host, the code is approved by this command as that account, and
///    the login is waited for. Nothing is left behind, and one line is
///    printed: the identity signed in, `{"issuer","subject"}`. The
///    rules below are every other issuer's.
/// 1. **Login** is the device authorization grant, and the one login
///    there is. The identity's lock is taken first — waited for while
///    another holds it — and held until the login ends. The user code
///    and where it is approved are printed as one line,
///    `{"user_code","verification_uri","verification_uri_complete",
///    "expires_in"}`, and the command ends there: no browser is
///    opened, and nothing is waited for.
/// 2. **The wait is left behind** — see [`detach`]: this command is
///    the front, and the login is its waiter's, which polls on after
///    the front has ended until the code is approved, denied or
///    expired.
/// 3. **The outcome is not printed.** An approval as the subject named
///    writes the identity — replacing one already there, signed out
///    or not — and `identities list` shows it; any other ending writes
///    nothing, and there is nothing to show.
/// 4. **List.** Every identity, `{"issuer","subject"}`, one line each.
/// 5. **Remove.** The identity taken out, printed as `{"removed":…}`.
///
/// No line holds a token.
pub async fn run(root: &Path, command: Identities) -> Result<(), Error> {
    // Rule 0.
    if let Identities::Login { issuer, subject, client_id, scope, resource } = &command
        && let Some(ours) = ours::ours(root, issuer, subject).await?
    {
        let store = identities::Identities::open(root).await?;
        let name = Name { issuer: issuer.clone(), subject: subject.clone() };
        let pending = store.login(&name, client_id, scope.as_deref().unwrap_or(""), resource.as_deref()).await?;
        ours.approve(&pending.user_code).await?;
        let name = pending.wait().await?;
        return print(&[json!({ "issuer": name.issuer, "subject": name.subject })]).await;
    }
    // Rule 2: the front does none of the login's work.
    if matches!(command, Identities::Login { .. }) && !detach::waiter() {
        return Ok(detach::relay().await?);
    }
    let store = identities::Identities::open(root).await?;
    match command {
        Identities::Login {
            issuer,
            subject,
            client_id,
            scope,
            resource,
        } => {
            let name = Name { issuer, subject };
            let scope = scope.as_deref().unwrap_or("");
            // Rule 1.
            let pending = store.login(&name, &client_id, scope, resource.as_deref()).await?;
            print(&[json!({
                "user_code": pending.user_code,
                "verification_uri": pending.verification_uri,
                "verification_uri_complete": pending.verification_uri_complete,
                "expires_in": pending.expires_in,
            })])
            .await?;
            detach::leave().await;
            // Rule 3: how the wait ended is the identity written, or
            // not; nobody is left to be told.
            let _ = pending.wait().await;
            Ok(())
        }
        Identities::List => {
            let lines: Vec<_> = store
                .list()
                .await?
                .into_iter()
                .map(|name| json!({ "issuer": name.issuer, "subject": name.subject }))
                .collect();
            print(&lines).await
        }
        Identities::Remove { issuer, subject } => {
            let name = Name { issuer, subject };
            store.remove(&name).await?;
            print(&[json!({ "removed": { "issuer": name.issuer, "subject": name.subject } })]).await
        }
    }
}
