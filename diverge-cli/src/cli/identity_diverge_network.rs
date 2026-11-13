//! The `identity.diverge.network` command, done.

use std::path::Path;

use diverge_sdk::identity_diverge_network::{self, Account};
use serde_json::json;

use super::Error;
use super::print::print;
use crate::arguments::{Accounts, IdentityDivergeNetwork, RefreshTokens, Subjects};
use crate::detach;

/// Do `command`, printing one JSON line per answer. The rules:
///
/// 1. **`accounts list`.** Every account on the host, one line each:
///    `{"account","issuer","id"}`, the account written
///    `<issuer>.<id>` — the first two parts of every subject it owns.
///    No file is read, and nothing is asked of anybody.
/// 2. **`accounts info`.** What the account's issuer says of its
///    user, asked now and kept nowhere, one line:
///    `{"account","issuer","id","info"}`. The `info` is the issuer's
///    own object, as it wrote it, so it is one shape for GitHub and
///    another for Google.
/// 3. **`accounts login`** is the device authorization grant at the
///    issuer named. The user code and the issuer's page are printed
///    as one line, `{"user_code","verification_uri",
///    "verification_uri_complete","expires_in","browser_opened"}`, and
///    the command ends there.
/// 4. **The browser.** Unless `--no-browser`, this host's browser is
///    opened at the issuer's page — the one with the code in it where
///    the issuer gives one — before the line is printed, and
///    `browser_opened` says whether a browser could be started. It
///    says no more than that: a browser started is not a page seen,
///    so the line names the page either way, for whoever reads it to
///    pass on. The code is always entered by a person: no issuer here
///    gives a page with the code in it.
/// 5. **The wait is left behind** — see [`detach`]: a login's command
///    is the front, and the login is its waiter's, which polls on
///    after the front has ended until the code is approved, denied or
///    expired.
/// 6. **A login's outcome is not printed.** An approval writes the
///    account of whoever approved — replacing one already there — and
///    `accounts list` shows it; any other ending writes nothing, and
///    there is nothing to show.
/// 7. **The requests of `identity.diverge.network`** — `subjects
///    list`, `refresh-tokens list` and `end` — are each one request
///    made as the account named, and print what it answers: one line
///    a subject, one a refresh token, and `{"ended":…}` for one
///    ended. A refusal is the error, with its
///    status, its code and what it said.
/// 8. **An account that cannot ask** — not on the host, or one its
///    issuer no longer answers for — is the error, and names what to
///    do: it logs in again.
///
/// No line holds a token.
pub async fn run(root: &Path, command: IdentityDivergeNetwork) -> Result<(), Error> {
    // Rule 5: the front does none of the login's work.
    if matches!(command, IdentityDivergeNetwork::Accounts(Accounts::Login { .. })) && !detach::waiter() {
        return Ok(detach::relay().await?);
    }
    let accounts = identity_diverge_network::Accounts::open(root).await?;
    match command {
        // Rule 1.
        IdentityDivergeNetwork::Accounts(Accounts::List) => {
            let lines: Vec<_> = accounts.list().await?.into_iter().map(|account| json!({ "account": account.to_string(), "issuer": account.issuer, "id": account.id })).collect();
            print(&lines).await
        }
        // Rule 2.
        IdentityDivergeNetwork::Accounts(Accounts::Info { issuer, id }) => {
            let account = Account { issuer, id };
            let info = accounts.info(&account).await?;
            print(&[json!({ "account": account.to_string(), "issuer": account.issuer, "id": account.id, "info": info })]).await
        }
        IdentityDivergeNetwork::Accounts(Accounts::Login { issuer, no_browser }) => {
            // Rule 3.
            let pending = accounts.login(&issuer).await?;
            // Rule 4.
            let page = pending.verification_uri_complete.as_deref().unwrap_or(&pending.verification_uri);
            let browser_opened = !no_browser && webbrowser::open(page).is_ok();
            print(&[json!({
                "user_code": pending.user_code,
                "verification_uri": pending.verification_uri,
                "verification_uri_complete": pending.verification_uri_complete,
                "expires_in": pending.expires_in,
                "browser_opened": browser_opened,
            })])
            .await?;
            detach::leave().await;
            // Rule 6: how the wait ended is the account written, or
            // not; nobody is left to be told.
            let _ = pending.wait().await;
            Ok(())
        }
        // Rule 7.
        IdentityDivergeNetwork::Subjects(Subjects::List { issuer, id }) => {
            let lines: Vec<_> = accounts.subjects(&Account { issuer, id }).await?.into_iter().map(|subject| json!(subject)).collect();
            print(&lines).await
        }
        IdentityDivergeNetwork::RefreshTokens(RefreshTokens::List { issuer, id, name }) => {
            let lines: Vec<_> = accounts.refresh_tokens(&Account { issuer, id }, &name).await?.into_iter().map(|token| json!(token)).collect();
            print(&lines).await
        }
        IdentityDivergeNetwork::RefreshTokens(RefreshTokens::End { issuer, id, name, refresh_token }) => {
            accounts.end(&Account { issuer, id }, &name, &refresh_token).await?;
            print(&[json!({ "ended": refresh_token })]).await
        }
    }
}
