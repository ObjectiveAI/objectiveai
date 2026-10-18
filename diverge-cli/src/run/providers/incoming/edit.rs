//! `diverge run providers incoming edit`: replacing a credential: a
//! patch to the identity it names and the address it is accepted from,
//! as
//! [`edit`](diverge_sdk::daemon::endpoints::providers::incoming::edit)
//! states it.

use clap::Args;
use diverge_sdk::daemon::endpoints::providers::incoming::edit::{
    client::request, server::response,
};

use crate::run::{Below, Emitter, Error, Target, body, request_schema, response_schema, shape};

/// Replacing a credential: a patch to the identity it names and the
/// address it is accepted from.
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
            return request_schema::<request::Frame>("diverge run providers incoming edit", emitter)
                .await;
        }
        Some(Below::ResponseSchema(_)) => {
            return response_schema::<response::Frame>(
                "diverge run providers incoming edit",
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
