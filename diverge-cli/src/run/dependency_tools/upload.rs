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
use crate::run::{Emitter, Error, body, shape};

/// Uploading files into a dependency tool's container.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub struct Arguments {
    /// What the request carries, as a JSON object: see
    /// [`request::Frame`]. Absent, `{}`.
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
    handle: &Handle,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    let request: request::Frame = body(arguments.body.as_deref())?;
    shape::upload(handle, &request, &arguments.local, emitter).await
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
