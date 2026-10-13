//! One run of the program.

use crate::arguments::{Arguments, Command};
use crate::config;
use super::Error;

/// Run `diverge` as told: the root found, `cli.json` read, the daemon
/// and the account selected, and the command done — which, for the
/// one command there is and for none, is nothing more yet: the
/// configuration is resolved and the run ends.
pub async fn run(arguments: Arguments) -> Result<(), Error> {
    let root = config::root(arguments.config).await?;
    let config = config::load(&root).await?;
    let selected = config::select(&config, arguments.daemon.as_deref(), arguments.account.as_deref())?;
    match arguments.command {
        None | Some(Command::Run) => {
            let _ = selected;
            Ok(())
        }
    }
}
