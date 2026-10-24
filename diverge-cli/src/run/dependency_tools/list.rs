//! `diverge run dependency-tools list`: listing a caller's dependency
//! tools, as
//! [`list`](diverge_sdk::daemon::endpoints::dependency_tools::list)
//! states it.

use clap::Args;
use diverge_sdk::daemon::endpoints::dependency_tools::list::{client::request, server::response};

use crate::run::{Below, Emitter, Error, Target, body, request_schema, response_schema, shape};

/// Listing a caller's dependency tools.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
#[command(subcommand_negates_reqs = true, args_conflicts_with_subcommands = true)]
pub struct Arguments {
    /// What the request carries, as a JSON object: see
    /// [`request::Frame`]; `relay` among its members has another daemon
    /// serve it. Absent, `{}`.
    #[arg(value_name = "BODY")]
    pub body: Option<String>,
    /// Keep the list open: after the word that the list is whole, every
    /// change printed as it comes, for as long as the command runs —
    /// for a program that reads the stream and ends the command when it
    /// is done. Absent, the command ends at that word.
    #[arg(long)]
    pub watch: bool,
    /// What is beneath the request: `request-schema`, the JSON Schema
    /// of the body it takes, or `response-schema`, the JSON Schema of
    /// every line it prints — printed in place of putting any request.
    #[command(subcommand)]
    pub below: Option<Below>,
}

/// The request put, every item printed as it comes, through the
/// word that the list is whole — and, with `--watch`, every change
/// after it, until the command is ended.
pub async fn run(
    arguments: Arguments,
    target: &Target<'_>,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    match arguments.below {
        Some(Below::RequestSchema(_)) => {
            return request_schema::<request::Frame>("diverge run dependency-tools list", emitter)
                .await;
        }
        Some(Below::ResponseSchema(_)) => {
            return response_schema::<response::Frame>("diverge run dependency-tools list", emitter)
                .await;
        }
        None => {}
    }
    let request: request::Frame = body(arguments.body.as_deref())?;
    let handle = target.dial().await?;
    let stop = if arguments.watch {
        shape::never
    } else {
        shape::at_listed
    };
    shape::stream::<_, response::Frame, _>(&handle, &request, emitter, stop).await
}
