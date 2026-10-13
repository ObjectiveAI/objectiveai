//! One run of the program.

use crate::arguments::{Arguments, Command};
use crate::config;
use super::Error;

/// Run `diverge` as told: the root found; then, for `run`, `cli.yaml`
/// read and the daemon and the account selected — and nothing more
/// yet: the request is not put, and the run ends; for `config`, the
/// document read, narrowed, rewritten or edited, see
/// [`config`](super::config). With no command the root is found and
/// the run ends.
pub async fn run(arguments: Arguments) -> Result<(), Error> {
    let root = config::root(arguments.config).await?;
    match arguments.command {
        None => Ok(()),
        Some(Command::Run(run)) => {
            let config = config::load(&root).await?;
            let selected = config::select(&config, run.daemon.as_deref(), run.account.as_deref())?;
            let _ = (selected, run.path, run.body);
            Ok(())
        }
        Some(Command::Config(request)) => super::config::run(&root, request.document).await,
    }
}
