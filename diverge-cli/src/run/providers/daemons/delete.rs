//! `diverge run providers daemons delete`: forgetting a daemon, as
//! [`delete`](diverge_sdk::daemon::endpoints::providers::daemons::delete)
//! states it.

use clap::Args;
use diverge_sdk::daemon::endpoints::providers::daemons::delete::{
    client::request, server::response,
};

use crate::run::{Below, Emitter, Error, Target, body, request_schema, shape};

/// Forgetting a daemon.
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

/// The request put, its one answer printed.
pub async fn run(
    arguments: Arguments,
    target: &Target<'_>,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    if let Some(Below::RequestSchema(_)) = arguments.below {
        return request_schema::<request::Frame>("diverge run providers daemons delete", emitter)
            .await;
    }
    let request: request::Frame = body(arguments.body.as_deref())?;
    let handle = target.dial().await?;
    shape::one_shot::<_, response::Frame, _>(&handle, &request, emitter).await
}
