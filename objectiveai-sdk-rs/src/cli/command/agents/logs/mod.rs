//! `agents logs` — persisted log tier. Currently one sub-tier:
//! `read`, which exposes `all`, `id`, `pending`, `subscribe`. The
//! `Request` / `ResponseItem` types alias straight through to
//! `read::*` — wrapping them in a single-variant enum would only
//! add noise.

pub mod read;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Read logged completion chunks. Sub-leaves: `all` (stream
    /// every row), `id` (look up a single row), `pending` (one-shot
    /// list of unfinalized rows), `subscribe` (long-lived stream of
    /// new rows).
    Read {
        #[command(subcommand)]
        command: read::Command,
    },
}

pub type Request = read::Request;
pub type ResponseItem = read::ResponseItem;

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Read { command } => read::Request::try_from(command),
        }
    }
}

#[cfg(feature = "cli-executor")]
pub use read::{execute, execute_jq};
