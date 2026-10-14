//! `diverge run standalone-tools transfer`: copying a file or a
//! directory out of a tool's container into an agent, a tool, a
//! dependency tool or a volume, without the client in between, as
//! [`transfer`](diverge_sdk::daemon::endpoints::standalone_tools::transfer)
//! states it.

use clap::Args;
use diverge_sdk::daemon::endpoints::standalone_tools::transfer::{
    client::request, server::response,
};
use diverge_sdk::wire::client::handle::Handle;

use crate::run::{Emitter, Error, body, shape};

/// Copying a file or a directory out of a tool's container into an
/// agent, a tool, a dependency tool or a volume, without the client in
/// between.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub struct Arguments {
    /// What the request carries, as a JSON object: see
    /// [`request::Frame`]. Absent, `{}`.
    #[arg(value_name = "BODY")]
    pub body: Option<String>,
}

/// The request put, its one answer printed.
pub async fn run(
    arguments: Arguments,
    handle: &Handle,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    let request: request::Frame = body(arguments.body.as_deref())?;
    shape::one_shot::<_, response::Frame, _>(handle, &request, emitter).await
}
