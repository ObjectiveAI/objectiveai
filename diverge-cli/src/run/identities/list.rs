//! `diverge run identities list`: listing the daemon's host's
//! identities, each with its tags, as
//! [`list`](diverge_sdk::daemon::endpoints::identities::list) states it.

use clap::Args;
use diverge_sdk::daemon::endpoints::identities::list::{client::request, server::response};

use crate::run::{Below, Emitter, Error, Target, body, request_schema, response_schema, shape};

/// Listing the identities, each with its tags.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
#[command(subcommand_negates_reqs = true, args_conflicts_with_subcommands = true)]
pub struct Arguments {
    /// What the request carries, as a JSON object: see
    /// [`request::Frame`]; `relay` among its members has another daemon
    /// serve it. Absent, `{}`.
    #[arg(value_name = "BODY")]
    pub body: Option<String>,
    /// Only identities at this issuer; given more than once, at any one
    /// of them. Added to the body's `issuers`.
    #[arg(long = "issuer", value_name = "ISSUER")]
    pub issuers: Vec<String>,
    /// Only identities of this subject; given more than once, any one of
    /// them. Added to the body's `subjects`.
    #[arg(long = "subject", value_name = "SUBJECT")]
    pub subjects: Vec<String>,
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
            return request_schema::<request::Frame>("diverge run identities list", emitter).await;
        }
        Some(Below::ResponseSchema(_)) => {
            return response_schema::<response::Frame>("diverge run identities list", emitter).await;
        }
        None => {}
    }
    let mut request: request::Frame = body(arguments.body.as_deref())?;
    request.filter.issuers.extend(arguments.issuers);
    request.filter.subjects.extend(arguments.subjects);
    let handle = target.dial().await?;
    shape::one_shot::<_, response::Frame, _>(&handle, &request, emitter).await
}
