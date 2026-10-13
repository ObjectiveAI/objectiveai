//! One run of the program.

use crate::arguments::{Arguments, Command};
use crate::config;
use super::Error;

/// Run `diverge` as told: the root found and `cli.json` read; then,
/// for `run`, the daemon and the account selected — and nothing more
/// yet: the request is not put, and the run ends. With no command the
/// configuration is resolved and the run ends.
pub async fn run(arguments: Arguments) -> Result<(), Error> {
    let root = config::root(arguments.config).await?;
    let config = config::load(&root).await?;
    match arguments.command {
        None => Ok(()),
        Some(Command::Run(run)) => {
            let selected = config::select(&config, run.daemon.as_deref(), run.account.as_deref())?;
            let _ = (selected, run.path, run.body);
            Ok(())
        }
    }
}
