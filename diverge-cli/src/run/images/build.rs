//! `diverge run images build`: an image built on a provider from a
//! Containerfile, into the daemon's Store there, as
//! [`build`](diverge_sdk::daemon::endpoints::images::build) states it.

use clap::Args;
use diverge_sdk::daemon::endpoints::images::build::{client::request, server::response};

use crate::run::{Below, Emitter, Error, Target, body, request_schema, response_schema, shape};

/// Building an image on a provider.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
#[command(subcommand_negates_reqs = true, args_conflicts_with_subcommands = true)]
pub struct Arguments {
    /// What the request carries, as a JSON object: see
    /// [`request::Frame`] — the provider, the Containerfile's text, a
    /// description, and the context, a directory of a volume on that
    /// provider; `relay` among its members has another daemon serve
    /// it. Absent, `{}`.
    #[arg(value_name = "BODY")]
    pub body: Option<String>,
    /// What is beneath the request: `request-schema`, the JSON Schema
    /// of the body it takes, or `response-schema`, the JSON Schema of
    /// every line it prints — printed in place of putting any request.
    #[command(subcommand)]
    pub below: Option<Below>,
}

/// The request put, and every answer printed as it comes, to the
/// daemon's finish: a line for each line the build says, then the one
/// line that ends it — the image built, with its digest, or why there
/// is none. The stream ends by itself, so nothing here cuts it.
pub async fn run(
    arguments: Arguments,
    target: &Target<'_>,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    match arguments.below {
        Some(Below::RequestSchema(_)) => {
            return request_schema::<request::Frame>("diverge run images build", emitter).await;
        }
        Some(Below::ResponseSchema(_)) => {
            return response_schema::<response::Frame>("diverge run images build", emitter).await;
        }
        None => {}
    }
    let request: request::Frame = body(arguments.body.as_deref())?;
    let handle = target.dial().await?;
    shape::stream::<_, response::Frame, _>(&handle, &request, emitter, shape::never).await
}
