//! The `identity.diverge.network` command, done.

use std::path::Path;

use diverge_sdk::identity_diverge_network;
use serde_json::json;

use super::Error;
use super::print::print;
use crate::arguments::{Accounts, IdentityDivergeNetwork};
use crate::detach;

/// Do `command` to the root's accounts at `identity.diverge.network`,
/// printing one JSON line per answer. The rules:
///
/// 1. **List.** Every account on the host, one line each:
///    `{"account","provider","id"}`, the account written
///    `<provider>.<id>` — the first two parts of every subject it
///    owns. No file is read, and nothing is asked of anybody.
/// 2. **Info.** What the account's provider says of its user, asked
///    now and kept nowhere, one line:
///    `{"account","provider","id","info"}`. The `info` is the
///    provider's own object, as it wrote it, so it is one shape for
///    GitHub and another for Google. An account not on the host, and
///    one its provider no longer answers for, are the error.
/// 3. **Login** is the device authorization grant at the provider
///    named. The user code and the provider's page are printed as one
///    line, `{"user_code","verification_uri",
///    "verification_uri_complete","expires_in","browser_opened"}`, and
///    the command ends there.
/// 4. **The browser.** Unless `--no-browser`, this host's browser is
///    opened at the provider's page — the one with the code in it
///    where the provider gives one — before the line is printed, and
///    `browser_opened` says whether a browser could be started. It
///    says no more than that: a browser started is not a page seen,
///    so the line names the page either way, for whoever reads it to
///    pass on. The code is always entered by a person: no provider
///    here gives a page with the code in it.
/// 5. **The wait is left behind** — see [`detach`]: this command is
///    the front, and the login is its waiter's, which polls on after
///    the front has ended until the code is approved, denied or
///    expired.
/// 6. **The outcome is not printed.** An approval writes the account
///    of whoever approved — replacing one already there — and `list`
///    shows it; any other ending writes nothing, and there is nothing
///    to show.
///
/// No line holds a token.
pub async fn run(root: &Path, command: IdentityDivergeNetwork) -> Result<(), Error> {
    let IdentityDivergeNetwork::Accounts(command) = command;
    // Rule 5: the front does none of the login's work.
    if matches!(command, Accounts::Login { .. }) && !detach::waiter() {
        return Ok(detach::relay().await?);
    }
    let accounts = identity_diverge_network::Accounts::open(root).await?;
    match command {
        // Rule 1.
        Accounts::List => {
            let lines: Vec<_> = accounts.list().await?.into_iter().map(|account| json!({ "account": account.to_string(), "provider": account.provider, "id": account.id })).collect();
            print(&lines).await
        }
        // Rule 2.
        Accounts::Info { provider, id } => {
            let account = identity_diverge_network::Account { provider, id };
            let info = accounts.info(&account).await?;
            print(&[json!({ "account": account.to_string(), "provider": account.provider, "id": account.id, "info": info })]).await
        }
        Accounts::Login { provider, no_browser } => {
            // Rule 3.
            let pending = accounts.login(&provider).await?;
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
    }
}
