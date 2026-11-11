//! `diverge run dependency-tools exec`: running one command in a dependency tool's
//! container, its output streamed, as
//! [`exec`](diverge_sdk::daemon::endpoints::dependency_tools::exec) states it.

use clap::Args;
use diverge_sdk::daemon::endpoints::dependency_tools::exec::{client::execute, client::request, server::response};

use crate::run::shape::exec::Said;
use crate::run::{Below, Emitter, Error, Target, body, request_schema, response_schema, shape};

/// Running one command in a dependency tool's container.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
#[command(subcommand_negates_reqs = true, args_conflicts_with_subcommands = true)]
pub struct Arguments {
    /// What the request carries, as a JSON object: see
    /// [`request::Frame`] — whose container, `argv`, and perhaps `cwd`
    /// and `env`; `relay` among its members has another daemon serve
    /// it.
    #[arg(value_name = "BODY")]
    pub body: Option<String>,
    /// The command's own output instead of JSON lines: its stdout to
    /// this one's stdout, its stderr to this one's stderr, and this
    /// command's exit the command's code.
    #[arg(long)]
    pub raw: bool,
    /// What is beneath the request: `request-schema`, the JSON Schema
    /// of the body it takes, or `response-schema`, the JSON Schema of
    /// every line it prints — printed in place of putting any request.
    #[command(subcommand)]
    pub below: Option<Below>,
}

/// The request put, and the command's stream printed until its end;
/// ending this command kills the one in the container.
pub async fn run(arguments: Arguments, target: &Target<'_>, emitter: &mut Emitter) -> Result<(), Error> {
    match arguments.below {
        Some(Below::RequestSchema(_)) => {
            return request_schema::<request::Frame>("diverge run dependency-tools exec", emitter).await;
        }
        Some(Below::ResponseSchema(_)) => {
            return response_schema::<response::Frame>("diverge run dependency-tools exec", emitter).await;
        }
        None => {}
    }
    let request: request::Frame = body(arguments.body.as_deref())?;
    let handle = target.dial().await?;
    let (answers, _cancel) = execute::execute(&handle, &request).await.map_err(shape::exchange::open)?;
    shape::exec::exec(answers, arguments.raw, emitter, said).await
}

/// What an answer says, for the raw printing.
fn said(frame: &response::Frame) -> Said {
    match frame {
        response::Frame::Stdout(bytes) => Said::Stdout(bytes.clone()),
        response::Frame::Stderr(bytes) => Said::Stderr(bytes.clone()),
        response::Frame::Exited(code) => Said::Exited(*code),
        response::Frame::Signaled(signal) => Said::Signaled(*signal),
        response::Frame::NotFound | response::Frame::Forbidden | response::Frame::Error(_) => Said::Refused,
    }
}
