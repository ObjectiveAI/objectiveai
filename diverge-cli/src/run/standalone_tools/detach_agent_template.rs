//! `diverge run standalone-tools detach-agent-template`: detaching a
//! tool from every agent of a template, by the tool's name and the
//! template's id, as
//! [`detach_agent_template`](diverge_sdk::daemon::endpoints::standalone_tools::detach_agent_template)
//! states it.

use clap::Args;
use diverge_sdk::daemon::endpoints::standalone_tools::detach_agent_template::{
    client::request, server::response,
};

use crate::run::{Below, Emitter, Error, Target, body, request_schema, response_schema, shape};

/// Detaching a tool from every agent of a template, by the tool's name
/// and the template's id.
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

/// The request put, its one answer printed.
pub async fn run(
    arguments: Arguments,
    target: &Target<'_>,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    match arguments.below {
        Some(Below::RequestSchema(_)) => {
            return request_schema::<request::Frame>(
                "diverge run standalone-tools detach-agent-template",
                emitter,
            )
            .await;
        }
        Some(Below::ResponseSchema(_)) => {
            return response_schema::<response::Frame>(
                "diverge run standalone-tools detach-agent-template",
                emitter,
            )
            .await;
        }
        None => {}
    }
    let request: request::Frame = body(arguments.body.as_deref())?;
    let handle = target.dial().await?;
    shape::one_shot::<_, response::Frame, _>(&handle, &request, emitter).await
}
