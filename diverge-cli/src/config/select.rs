//! Which daemon, and which account of it.

use super::{Account, Config, Daemon, Error};

/// The daemon and the account a run is: what the command line named,
/// else the document's defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selected<'a> {
    /// The daemon's name in the document.
    pub daemon_name: &'a str,
    /// The daemon.
    pub daemon: &'a Daemon,
    /// The account's name under the daemon.
    pub account_name: &'a str,
    /// The account.
    pub account: &'a Account,
}

/// The daemon `daemon` names, else the document's default, and the
/// account `account` names under it, else the daemon's default. A
/// name the document does not have is [`Error::NoDaemon`] or
/// [`Error::NoAccount`].
pub fn select<'a>(
    config: &'a Config,
    daemon: Option<&'a str>,
    account: Option<&'a str>,
) -> Result<Selected<'a>, Error> {
    let daemon_name = daemon.unwrap_or(&config.default);
    let Some(daemon) = config.daemons.get(daemon_name) else {
        return Err(Error::NoDaemon(daemon_name.to_string()));
    };
    let account_name = account.unwrap_or(&daemon.default);
    let Some(account) = daemon.accounts.get(account_name) else {
        return Err(Error::NoAccount {
            daemon: daemon_name.to_string(),
            account: account_name.to_string(),
        });
    };
    Ok(Selected {
        daemon_name,
        daemon,
        account_name,
        account,
    })
}
