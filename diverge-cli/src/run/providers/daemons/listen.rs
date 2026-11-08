//! `diverge run providers daemons listen`: listening for the messages sent to the
//! daemon, as
//! [`listen`](diverge_sdk::daemon::endpoints::providers::daemons::listen) states
//! it.

use clap::Args;
use diverge_sdk::daemon::endpoints::providers::daemons::listen::{client::request, server::response};

use crate::run::{Below, Emitter, Error, Target, body, request_schema, response_schema, shape};

/// Listening for the messages sent to the daemon through its providers.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
#[command(subcommand_negates_reqs = true, args_conflicts_with_subcommands = true)]
pub struct Arguments {
    /// What the request carries, as a JSON object: see
    /// [`request::Frame`]; `relay` among its members has another daemon
    /// serve it. Absent, `{}`.
    #[arg(value_name = "BODY")]
    pub body: Option<String>,
    /// What is beneath the request: `request-schema`, the JSON Schema
    /// of the body it takes, or `response-schema`, the JSON Schema of
    /// every line it prints — printed in place of putting any request.
    #[command(subcommand)]
    pub below: Option<Below>,
}

/// The request put, and every message printed as it comes — its body in
/// standard base64 — for as long as the command runs: a listen has
/// nothing to list once, so it ends only when the command is ended, or
/// at the daemon's finish.
pub async fn run(
    arguments: Arguments,
    target: &Target<'_>,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    match arguments.below {
        Some(Below::RequestSchema(_)) => {
            return request_schema::<request::Frame>("diverge run providers daemons listen", emitter).await;
        }
        Some(Below::ResponseSchema(_)) => {
            return response_schema::<response::Frame>("diverge run providers daemons listen", emitter).await;
        }
        None => {}
    }
    let request: request::Frame = body(arguments.body.as_deref())?;
    let handle = target.dial().await?;
    shape::stream::<_, response::Frame, _>(&handle, &request, emitter, shape::never).await
}
