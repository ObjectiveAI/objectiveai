//! `diverge run dependency-tools upload`: uploading files into a
//! dependency tool's container, as
//! [`upload`](diverge_sdk::daemon::endpoints::dependency_tools::upload)
//! states it.

use std::path::PathBuf;

use clap::Args;
use diverge_sdk::daemon::endpoints::dependency_tools::upload::{client::execute, client::request};
use diverge_sdk::wire::client::handle::Handle;
use serde_json::Value;

use crate::run::shape::{Content, Upload};
use crate::run::{Below, Emitter, Error, Target, body, request_schema, shape};

/// Uploading files into a dependency tool's container.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
#[command(subcommand_negates_reqs = true, args_conflicts_with_subcommands = true)]
pub struct Arguments {
    /// What the request carries, as a JSON object: see
    /// [`request::Frame`]; `relay` among its members has another daemon
    /// serve it, as an account held there. Absent, `{}`.
    #[arg(value_name = "BODY")]
    pub body: Option<String>,
    /// The local file a file upload reads, or the local directory a
    /// directory upload reads each named file under.
    #[arg(long = "path", value_name = "LOCAL")]
    pub local: PathBuf,
    /// What is beneath the request: `request-schema`, the JSON Schema
    /// of the body it takes, printed in place of putting any request.
    #[command(subcommand)]
    pub below: Option<Below>,
}

/// The request put, every file the daemon asks for read from under
/// the local path, and the answer printed.
pub async fn run(
    arguments: Arguments,
    target: &Target<'_>,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    if let Some(Below::RequestSchema(_)) = arguments.below {
        return request_schema::<request::Frame>("diverge run dependency-tools upload", emitter)
            .await;
    }
    let request: request::Frame = body(arguments.body.as_deref())?;
    let handle = target.dial().await?;
    shape::upload(&handle, &request, &arguments.local, emitter).await
}

impl Upload for request::Frame {
    async fn execute<F>(&self, handle: &Handle, source: F) -> Result<Value, Error>
    where
        F: FnMut(Option<String>) -> Option<Content> + Send,
    {
        let answer = execute::execute(handle, self, source)
            .await
            .map_err(shape::exchange::upload)?;
        serde_json::to_value(&answer).map_err(Error::Json)
    }
}
