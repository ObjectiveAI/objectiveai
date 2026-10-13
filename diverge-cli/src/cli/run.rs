//! One run of the program.

use crate::arguments::{Arguments, Command};
use crate::config;
use super::Error;

/// Run `diverge` as told: the root found; then, for `run`, `cli.yaml`
/// read, the daemon and the account selected, and the request put —
/// see [`run`](crate::run); for `config`, the document read, narrowed,
/// rewritten or edited — see [`config`](super::config). With no
/// command the root is found and the run ends.
pub async fn run(arguments: Arguments) -> Result<(), Error> {
    let root = config::root(arguments.config).await?;
    match arguments.command {
        None => Ok(()),
        Some(Command::Run(run)) => {
            let config = config::load(&root).await?;
            let (daemon, account) = (run.daemon.clone(), run.account.clone());
            let selected = config::select(&config, daemon.as_deref(), account.as_deref())?;
            crate::run::run(&selected, run).await?;
            Ok(())
        }
        Some(Command::Config(request)) => super::config::run(&root, request.document).await,
    }
}
