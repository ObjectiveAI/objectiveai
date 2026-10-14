//! `diverge run postgres get`: reading which database the daemon
//! serves, as [`get`](diverge_sdk::daemon::endpoints::postgres::get)
//! states it.

use clap::Args;
use diverge_sdk::daemon::endpoints::postgres::get::{client::request, server::response};

use crate::run::{Emitter, Error, Target, shape};

/// Reading which database the daemon serves. There is exactly one,
/// so the request names nothing and takes no body: it is `{}` on the
/// wire.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub struct Arguments {}

/// The request put, its one answer printed.
pub async fn run(
    Arguments {}: Arguments,
    target: &Target<'_>,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    let request = request::Frame {};
    let handle = target.dial().await?;
    shape::one_shot::<_, response::Frame, _>(&handle, &request, emitter).await
}
