//! One run of the program.

use super::Error;
use crate::arguments::{Arguments, Command};
use crate::config;
use crate::lifecycle::{self, Program};

/// Run `diverge` as told: the root found; then, for `run`, `cli.yaml`
/// read, the daemon and the account selected, and the request put —
/// see [`run`](crate::run); for `config`, the document read, narrowed,
/// rewritten or edited — see [`config`](super::config); for `keyring`,
/// the root's keyring worked on — see [`keyring`](super::keyring); for
/// `link`, the daemon linked to the provider on the root — see
/// [`link`](super::link); for `daemon`
/// and `provider`, the program started on the root or stopped — see
/// [`lifecycle`]; for `issuer`, the issuer started or stopped, or its
/// identities and clients worked on — see [`issuer`](super::issuer). With no command the root is found and the run ends.
pub async fn run(arguments: Arguments) -> Result<(), Error> {
    let root = config::root(arguments.config).await?;
    match arguments.command {
        None => Ok(()),
        Some(Command::Run(run)) => {
            let config = config::load(&root).await?;
            let (daemon, account) = (run.daemon.clone(), run.account.clone());
            let selected = config::select(&config, daemon.as_deref(), account.as_deref())?;
            crate::run::run(&root, &selected, run).await?;
            Ok(())
        }
        Some(Command::Config(request)) => super::config::run(&root, request.document).await,
        Some(Command::Keyring(command)) => super::keyring::run(&root, command).await,
        Some(Command::Link(command)) => super::link::run(&root, command).await,
        Some(Command::Daemon(action)) => {
            lifecycle::run(&root, Program::Daemon, action).await?;
            Ok(())
        }
        Some(Command::Provider(action)) => {
            lifecycle::run(&root, Program::Provider, action).await?;
            Ok(())
        }
        Some(Command::Issuer(command)) => super::issuer::run(&root, command).await,
    }
}
