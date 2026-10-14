//! `diverge run agents logs`: an agent's log: everything its
//! conversation has carried, read, as
//! [`logs`](diverge_sdk::daemon::endpoints::agents::logs) states it.

use clap::Args;
use diverge_sdk::daemon::endpoints::agents::logs::{client::request, server::response};

use crate::run::{Below, Emitter, Error, Target, body, request_schema, shape};

/// An agent's log: everything its conversation has carried, read.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
#[command(subcommand_negates_reqs = true, args_conflicts_with_subcommands = true)]
pub struct Arguments {
    /// What the request carries, as a JSON object: see
    /// [`request::Frame`]; `relay` among its members has another daemon
    /// serve it, as an account held there. Absent, `{}`.
    #[arg(value_name = "BODY")]
    pub body: Option<String>,
    /// What is beneath the request: `request-schema`, the JSON Schema
    /// of the body it takes, printed in place of putting any request.
    #[command(subcommand)]
    pub below: Option<Below>,
}

/// The request put, every item printed as it comes, to the log's
/// end or, watching, as far as the request asked.
pub async fn run(
    arguments: Arguments,
    target: &Target<'_>,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    if let Some(Below::RequestSchema(_)) = arguments.below {
        return request_schema::<request::Frame>("diverge run agents logs", emitter).await;
    }
    let request: request::Frame = body(arguments.body.as_deref())?;
    let handle = target.dial().await?;
    shape::stream::<_, response::Frame, _>(&handle, &request, emitter, shape::never).await
}
