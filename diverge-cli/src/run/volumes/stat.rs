//! `diverge run volumes stat`: walking a volume: how much of it is
//! used, and the hash of its content, as
//! [`stat`](diverge_sdk::daemon::endpoints::volumes::stat) states it.

use clap::Args;
use diverge_sdk::daemon::endpoints::volumes::stat::{client::request, server::response};
use diverge_sdk::wire::client::handle::Handle;

use crate::run::{Emitter, Error, body, shape};

/// Walking a volume: how much of it is used, and the hash of its
/// content.
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
