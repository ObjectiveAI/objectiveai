//! `diverge run agents filetree`: watching an agent's container's
//! filesystem, whole, as
//! [`filetree`](diverge_sdk::daemon::endpoints::agents::filetree)
//! states it.

use clap::Args;
use diverge_sdk::daemon::endpoints::agents::filetree::{client::request, server::response};

use crate::run::{Below, Emitter, Error, Target, body, request_schema, response_schema, shape};

/// Watching an agent's container's filesystem, whole.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
#[command(subcommand_negates_reqs = true, args_conflicts_with_subcommands = true)]
pub struct Arguments {
    /// What the request carries, as a JSON object: see
    /// [`request::Frame`]; `relay` among its members has another daemon
    /// serve it, as an account held there. Absent, `{}`.
    #[arg(value_name = "BODY")]
    pub body: Option<String>,
    /// What is beneath the request: `request-schema`, the JSON Schema
    /// of the body it takes, or `response-schema`, the JSON Schema of
    /// every line it prints — printed in place of putting any request.
    #[command(subcommand)]
    pub below: Option<Below>,
}

/// The request put, the tree's snapshot printed; the changes after
/// it are not this command's.
pub async fn run(
    arguments: Arguments,
    target: &Target<'_>,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    match arguments.below {
        Some(Below::RequestSchema(_)) => {
            return request_schema::<request::Frame>("diverge run agents filetree", emitter).await;
        }
        Some(Below::ResponseSchema(_)) => {
            return response_schema::<response::Frame>("diverge run agents filetree", emitter)
                .await;
        }
        None => {}
    }
    let request: request::Frame = body(arguments.body.as_deref())?;
    let handle = target.dial().await?;
    shape::stream::<_, response::Frame, _>(&handle, &request, emitter, shape::at_snapshot).await
}
