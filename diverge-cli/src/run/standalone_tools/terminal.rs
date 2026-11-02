//! `diverge run standalone-tools terminal`: an interactive shell in a standalone tool's
//! container, wired to this command's stdin and stdout, as
//! [`terminal`](diverge_sdk::daemon::endpoints::standalone_tools::terminal) states it.

use clap::Args;
use diverge_sdk::daemon::endpoints::standalone_tools::terminal::{client::execute, client::request, server::response};

use crate::run::shape::terminal::Said;
use crate::run::{Below, Emitter, Error, Target, body, request_schema, response_schema, shape};

/// An interactive shell in a standalone tool's container.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
#[command(subcommand_negates_reqs = true, args_conflicts_with_subcommands = true)]
pub struct Arguments {
    /// What the request carries, as a JSON object: see
    /// [`request::Frame`] — whose container, and the terminal's `cols`
    /// and `rows`; `relay` among its members has another daemon serve
    /// it.
    #[arg(value_name = "BODY")]
    pub body: Option<String>,
    /// What is beneath the request: `request-schema`, the JSON Schema
    /// of the body it takes, or `response-schema`, the JSON Schema of
    /// every answer — printed in place of putting any request.
    #[command(subcommand)]
    pub below: Option<Below>,
}

/// The request put, this command's stdin fed to the shell and what it
/// prints written raw, until the shell ends (see
/// [`shape::terminal`](crate::run::shape::terminal)); this command
/// exits with the shell's code.
pub async fn run(arguments: Arguments, target: &Target<'_>, emitter: &mut Emitter) -> Result<(), Error> {
    match arguments.below {
        Some(Below::RequestSchema(_)) => {
            return request_schema::<request::Frame>("diverge run standalone-tools terminal", emitter).await;
        }
        Some(Below::ResponseSchema(_)) => {
            return response_schema::<response::Frame>("diverge run standalone-tools terminal", emitter).await;
        }
        None => {}
    }
    let request: request::Frame = body(arguments.body.as_deref())?;
    let handle = target.dial().await?;
    let (answers, terminal) = execute::execute(&handle, &request).await.map_err(shape::exchange::open)?;
    let stdin = move |bytes: Vec<u8>| {
        let terminal = terminal.clone();
        async move { terminal.stdin(&bytes).await.is_ok() }
    };
    shape::terminal::terminal(answers, stdin, said).await
}

/// What an answer says.
fn said(frame: &response::Frame) -> Said {
    match frame {
        response::Frame::Stdout(bytes) => Said::Stdout(bytes.clone()),
        response::Frame::Exited(code) => Said::Exited(*code),
        response::Frame::Signaled(signal) => Said::Signaled(*signal),
        response::Frame::NotFound | response::Frame::Forbidden | response::Frame::Error(_) => Said::Refused,
    }
}
