//! `diverge run standalone-tools filetree`: watching a tool's
//! container's filesystem, whole, as
//! [`filetree`](diverge_sdk::daemon::endpoints::standalone_tools::filetree)
//! states it.

use clap::Args;
use diverge_sdk::daemon::endpoints::standalone_tools::filetree::{
    client::request, server::response,
};
use diverge_sdk::wire::client::handle::Handle;

use crate::run::{Emitter, Error, body, shape};

/// Watching a tool's container's filesystem, whole.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub struct Arguments {
    /// What the request carries, as a JSON object: see
    /// [`request::Frame`]. Absent, `{}`.
    #[arg(value_name = "BODY")]
    pub body: Option<String>,
}

/// The request put, the tree's snapshot printed; the changes after
/// it are not this command's.
pub async fn run(
    arguments: Arguments,
    handle: &Handle,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    let request: request::Frame = body(arguments.body.as_deref())?;
    shape::stream::<_, response::Frame, _>(handle, &request, emitter, shape::at_snapshot).await
}
