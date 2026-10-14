//! `diverge run agents download`: downloading a file or a directory out
//! of an agent, as
//! [`download`](diverge_sdk::daemon::endpoints::agents::download)
//! states it.

use std::path::PathBuf;

use bytes::Bytes;
use clap::Args;
use diverge_sdk::daemon::client::stream::{ExecuteStream, OpenError};
use diverge_sdk::daemon::endpoints::agents::download::{
    client::execute, client::request, server::response,
};
use diverge_sdk::wire::client::handle::Handle;

use crate::run::shape::{Download, Downloaded};
use crate::run::{Emitter, Error, body, shape};

/// Downloading a file or a directory out of an agent.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub struct Arguments {
    /// What the request carries, as a JSON object: see
    /// [`request::Frame`]. Absent, `{}`.
    #[arg(value_name = "BODY")]
    pub body: Option<String>,
    /// The local file a file download is written to, or the local
    /// directory a directory download is written under, every parent
    /// made.
    #[arg(long = "path", value_name = "LOCAL")]
    pub local: PathBuf,
}

/// The request put, every file written under the local path, one
/// line per file written printed, and every other answer as it comes.
pub async fn run(
    arguments: Arguments,
    handle: &Handle,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    let request: request::Frame = body(arguments.body.as_deref())?;
    shape::download(handle, &request, &arguments.local, emitter).await
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
