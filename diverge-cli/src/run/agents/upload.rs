//! `diverge run agents upload`: uploading files into an agent's
//! container, as
//! [`upload`](diverge_sdk::daemon::endpoints::agents::upload) states
//! it.

use std::path::PathBuf;

use clap::Args;
use diverge_sdk::daemon::endpoints::agents::upload::{
    client::execute, client::request, server::response,
};
use diverge_sdk::wire::client::handle::Handle;

use crate::run::shape::{Content, Upload};
use crate::run::{Below, Emitter, Error, Target, body, request_schema, response_schema, shape};

/// Uploading files into an agent's container.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
#[command(subcommand_negates_reqs = true, args_conflicts_with_subcommands = true)]
pub struct Arguments {
    /// The request itself: its body and the local path. Absent when a
    /// schema is asked for beneath, whose arguments these are not.
    #[command(flatten)]
    pub regular: Option<Regular>,
    /// What is beneath the request: `request-schema`, the JSON Schema
    /// of the body it takes, or `response-schema`, the JSON Schema of
    /// every line it prints — printed in place of putting any request.
    #[command(subcommand)]
    pub below: Option<Below>,
}

/// The request itself, as it is put: the body and the local path —
/// one group, so that a schema asked for beneath needs neither, and
/// a request without `--path` does not parse.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub struct Regular {
    /// What the request carries, as a JSON object: see
    /// [`request::Frame`]; `relay` among its members has another daemon
    /// serve it, as an account held there. Absent, `{}`.
    #[arg(value_name = "BODY")]
    pub body: Option<String>,
    /// The local file a file upload reads, or the local directory a
    /// directory upload reads each named file under.
    #[arg(long = "path", value_name = "LOCAL")]
    pub local: PathBuf,
}

/// The request put, every file the daemon asks for read from under
/// the local path, and the answer printed.
pub async fn run(
    arguments: Arguments,
    target: &Target<'_>,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    match arguments.below {
        Some(Below::RequestSchema(_)) => {
            return request_schema::<request::Frame>("diverge run agents upload", emitter).await;
        }
        Some(Below::ResponseSchema(_)) => {
            return response_schema::<response::Frame>("diverge run agents upload", emitter).await;
        }
        None => {}
    }
    let Some(regular) = arguments.regular else {
        return Err(Error::NoRequest);
    };
    let request: request::Frame = body(regular.body.as_deref())?;
    let handle = target.dial().await?;
    shape::upload(&handle, &request, &regular.local, emitter).await
}

impl Upload for request::Frame {
    type Answer = response::Frame;

    async fn execute<F>(&self, handle: &Handle, source: F) -> Result<response::Frame, Error>
    where
        F: FnMut(Option<String>) -> Option<Content> + Send,
    {
        execute::execute(handle, self, source)
            .await
            .map_err(shape::exchange::upload)
    }
}
