//! `diverge run dependency-tools download`: downloading a file or a
//! directory out of a dependency tool, as
//! [`download`](diverge_sdk::daemon::endpoints::dependency_tools::download)
//! states it.

use std::path::PathBuf;

use bytes::Bytes;
use clap::Args;
use diverge_sdk::daemon::client::stream::{ExecuteStream, OpenError};
use diverge_sdk::daemon::endpoints::dependency_tools::download::{
    client::execute, client::request, server::response,
};
use diverge_sdk::wire::client::handle::Handle;

use crate::run::shape::{Download, Downloaded};
use crate::run::{Below, Emitter, Error, Target, body, request_schema, shape};

/// Downloading a file or a directory out of a dependency tool.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
#[command(subcommand_negates_reqs = true, args_conflicts_with_subcommands = true)]
pub struct Arguments {
    /// What the request carries, as a JSON object: see
    /// [`request::Frame`]; `relay` among its members has another daemon
    /// serve it, as an account held there. Absent, `{}`.
    #[arg(value_name = "BODY")]
    pub body: Option<String>,
    /// The local file a file download is written to, or the local
    /// directory a directory download is written under, every parent
    /// made.
    #[arg(long = "path", value_name = "LOCAL")]
    pub local: PathBuf,
    /// What is beneath the request: `request-schema`, the JSON Schema
    /// of the body it takes, printed in place of putting any request.
    #[command(subcommand)]
    pub below: Option<Below>,
}

/// The request put, every file written under the local path, one
/// line per file written printed, and every other answer as it comes.
pub async fn run(
    arguments: Arguments,
    target: &Target<'_>,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    if let Some(Below::RequestSchema(_)) = arguments.below {
        return request_schema::<request::Frame>("diverge run dependency-tools download", emitter)
            .await;
    }
    let request: request::Frame = body(arguments.body.as_deref())?;
    let handle = target.dial().await?;
    shape::download(&handle, &request, &arguments.local, emitter).await
}

impl Download for request::Frame {
    type Item = execute::Item;
    type Failure = response::FrameError;

    async fn execute(
        &self,
        handle: &Handle,
    ) -> Result<ExecuteStream<Self::Item, Self::Failure>, OpenError<serde_json::Error>> {
        execute::execute(handle, self).await
    }
}

impl Downloaded for execute::Item {
    fn chunk(self) -> Result<(Vec<String>, Bytes), Self> {
        match self {
            execute::Item::Chunk { path, body } => Ok((path, body)),
            other => Err(other),
        }
    }
}
