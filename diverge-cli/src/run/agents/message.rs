//! `diverge run agents message`: asking an agent something — a chat
//! message, or a structured decision — as
//! [`message`](diverge_sdk::daemon::endpoints::agents::message) states
//! it.

use clap::Args;
use diverge_sdk::daemon::endpoints::agents::message::{client::request, server::response};

use crate::run::{Below, Emitter, Error, Target, body, request_schema, response_schema, shape};

/// Asking an agent something: a chat message, or a structured
/// decision.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
#[command(subcommand_negates_reqs = true, args_conflicts_with_subcommands = true)]
pub struct Arguments {
    /// What the request carries, as a JSON object: see
    /// [`request::Frame`] — the agent, the `type` and that kind's
    /// members, and `watch`; `relay` among its members has another
    /// daemon serve it. Absent, `{}`.
    #[arg(value_name = "BODY")]
    pub body: Option<String>,
    /// What is beneath the request: `request-schema`, the JSON Schema
    /// of the body it takes, or `response-schema`, the JSON Schema of
    /// every line it prints — printed in place of putting any request.
    #[command(subcommand)]
    pub below: Option<Below>,
}

/// The request put, and every answer printed as it comes, to the
/// daemon's finish: one line, the request's fate, where it does not
/// watch; and where it does, a line for each item the agent's log
/// takes of it, from its echo to the end of its run — or the one line
/// that it has none. The stream ends by itself either way, so nothing
/// here cuts it.
pub async fn run(
    arguments: Arguments,
    target: &Target<'_>,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    match arguments.below {
        Some(Below::RequestSchema(_)) => {
            return request_schema::<request::Frame>("diverge run agents message", emitter).await;
        }
        Some(Below::ResponseSchema(_)) => {
            return response_schema::<response::Frame>("diverge run agents message", emitter).await;
        }
        None => {}
    }
    let request: request::Frame = body(arguments.body.as_deref())?;
    let handle = target.dial().await?;
    shape::stream::<_, response::Frame, _>(&handle, &request, emitter, shape::never).await
}
