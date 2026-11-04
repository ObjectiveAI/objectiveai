//! `diverge run identities subjects`: listing the subjects signed in at
//! one issuer, as
//! [`subjects`](diverge_sdk::daemon::endpoints::identities::subjects)
//! states it.

use clap::Args;
use diverge_sdk::daemon::endpoints::identities::subjects::{client::request, server::response};

use crate::run::{Below, Emitter, Error, Target, body, request_schema, response_schema, shape};

/// Listing the subjects at one issuer.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
#[command(subcommand_negates_reqs = true, args_conflicts_with_subcommands = true)]
pub struct Arguments {
    /// What the request carries, as a JSON object: see
    /// [`request::Frame`]; `relay` among its members has another daemon
    /// serve it. Absent, the issuer alone.
    #[arg(value_name = "BODY")]
    pub body: Option<String>,
    /// The issuer, exactly as its tokens' `iss` names it: in place of
    /// the body's `issuer`.
    #[arg(long, value_name = "ISSUER")]
    pub issuer: Option<String>,
    /// What is beneath the request: `request-schema`, the JSON Schema
    /// of the body it takes, or `response-schema`, the JSON Schema of
    /// every line it prints — printed in place of putting any request.
    #[command(subcommand)]
    pub below: Option<Below>,
}

/// The request put, its one answer printed. `--issuer` with no body is
/// the request; with a body, it replaces the body's `issuer`.
pub async fn run(
    arguments: Arguments,
    target: &Target<'_>,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    match arguments.below {
        Some(Below::RequestSchema(_)) => {
            return request_schema::<request::Frame>("diverge run identities subjects", emitter).await;
        }
        Some(Below::ResponseSchema(_)) => {
            return response_schema::<response::Frame>("diverge run identities subjects", emitter).await;
        }
        None => {}
    }
    let request: request::Frame = match (arguments.body.as_deref(), arguments.issuer) {
        (None, Some(issuer)) => request::Frame { issuer, relay: None },
        (body_text, issuer) => {
            let mut request: request::Frame = body(body_text)?;
            if let Some(issuer) = issuer {
                request.issuer = issuer;
            }
            request
        }
    };
    let handle = target.dial().await?;
    shape::one_shot::<_, response::Frame, _>(&handle, &request, emitter).await
}
