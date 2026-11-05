//! The `identities` command, done.

use std::path::Path;

use diverge_sdk::identities::{self, Name};
use serde_json::json;

use super::Error;
use super::print::print;
use crate::arguments::Identities;

/// Do `command` to the identities at the root, printing one JSON line
/// per answer. The rules:
///
/// 1. **Login.** The identity's lock is taken first — waited for while
///    another holds it — and held until the login ends, approved or
///    not. Then the user code and where to enter it are printed as one
///    line, `{"user_code","verification_uri",
///    "verification_uri_complete","expires_in"}`, and the browser is
///    opened at the complete URI where the issuer gives one, else at
///    the verification URI. A browser that will not open is no error:
///    the line names the page. The approval, as the subject named, is
///    written — replacing an identity already there, signed out or not
///    — and printed as `{"issuer","subject"}`; any other ending is the
///    error.
/// 4. **Login by password.** With `--password-stdin`, the password is
///    read from stdin — an empty one refused — and sent to the issuer
///    once, the password grant: no code, no browser. The identity, as
///    the subject named, is written and printed as
///    `{"issuer","subject"}`; any other ending is the error. Diverge's
///    own issuer signs in this way.
/// 2. **List.** Every identity, `{"issuer","subject"}`, one line each.
/// 3. **Remove.** The identity taken out, printed as `{"removed":…}`.
///
/// No line holds a token.
pub async fn run(root: &Path, command: Identities) -> Result<(), Error> {
    let store = identities::Identities::open(root).await?;
    match command {
        Identities::Login {
            issuer,
            subject,
            client_id,
            scope,
            resource,
            password_stdin,
        } => {
            let name = Name { issuer, subject };
            // Rule 4.
            if password_stdin {
                let password = super::issuer::read_password().await?;
                let name = store
                    .login_password(&name, &client_id, &password, scope.as_deref().unwrap_or(""), resource.as_deref())
                    .await?;
                return print(&[json!({ "issuer": name.issuer, "subject": name.subject })]).await;
            }
            let pending = store
                .login(
                    &name,
                    &client_id,
                    scope.as_deref().unwrap_or(""),
                    resource.as_deref(),
                )
                .await?;
            print(&[json!({
                "user_code": pending.user_code,
                "verification_uri": pending.verification_uri,
                "verification_uri_complete": pending.verification_uri_complete,
                "expires_in": pending.expires_in,
            })])
            .await?;
            let page = pending
                .verification_uri_complete
                .as_deref()
                .unwrap_or(&pending.verification_uri);
            let _ = webbrowser::open(page);
            let name = pending.wait().await?;
            print(&[json!({ "issuer": name.issuer, "subject": name.subject })]).await
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
