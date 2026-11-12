//! One run of the program.

use super::Error;
use crate::arguments::{Arguments, Command, Document, Server};
use crate::config;
use crate::document::Which;
use crate::lifecycle::{self, Program};

/// Run `diverge` as told: the root found; then, for `run`, `cli.yaml`
/// read, the daemon and the account selected, and the request put —
/// see [`run`](crate::run); for `<program> config`, the program's
/// document read, narrowed, rewritten or edited — see
/// [`config`](super::config); for `identities`, the root's identities
/// signed in, listed or removed — see [`identities`](super::identities);
/// for `daemon` and `provider`, the program started on the root or
/// stopped — see [`lifecycle`].
/// With no command the root is found and the run ends.
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
        Some(Command::Identities(command)) => super::identities::run(&root, command).await,
        Some(Command::Daemon(command)) => {
            server(&root, Program::Daemon, Which::Daemon, command).await
        }
        Some(Command::Provider(command)) => {
            server(&root, Program::Provider, Which::Provider, command).await
        }
        Some(Command::Viewer(Document::Config(action))) => {
            super::config::run(&root, Which::Viewer, action).await
        }
        Some(Command::Cli(Document::Config(action))) => {
            super::config::run(&root, Which::Cli, action).await
        }
    }
}

/// A server's command, done: `start` and `stop` — see [`lifecycle`] —
/// or `config` on its document — see [`config`](super::config).
async fn server(
    root: &std::path::Path,
    program: Program,
    which: Which,
    command: Server,
) -> Result<(), Error> {
    match command {
        Server::Lifecycle(action) => Ok(lifecycle::run(root, program, action).await?),
        Server::Config(action) => super::config::run(root, which, action).await,
    }
}
