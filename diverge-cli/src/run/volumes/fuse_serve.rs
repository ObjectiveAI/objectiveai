//! `diverge run volumes fuse-serve`: a volume served over FUSE through
//! the daemon, as
//! [`fuse_serve`](diverge_sdk::daemon::endpoints::volumes::fuse_serve)
//! states it — a request this program states and does not put.

use clap::Args;
use diverge_sdk::daemon::endpoints::volumes::fuse_serve::{client::request, server::response};

use crate::run::{Below, Emitter, Error, Target, body, request_schema, response_schema};

/// A volume served over FUSE through the daemon. Not supported from
/// the command line: see [`run`].
#[derive(Debug, Clone, PartialEq, Eq, Args)]
#[command(subcommand_negates_reqs = true, args_conflicts_with_subcommands = true)]
pub struct Arguments {
    /// What the request carries, as a JSON object: see
    /// [`request::Frame`] — the volume, the mode it is meant to be in,
    /// and the size of an ephemeral volume's layer; `relay` among its
    /// members has another daemon serve it. Read, and never put.
    #[arg(value_name = "BODY")]
    pub body: Option<String>,
    /// What is beneath the request: `request-schema`, the JSON Schema
    /// of the body it takes, or `response-schema`, the JSON Schema of
    /// its first answer — printed in place of putting any request.
    #[command(subcommand)]
    pub below: Option<Below>,
}

/// The request refused, here, by this program. The rules:
///
/// 1. **The schemas are answered.** `request-schema` and
///    `response-schema` print what the SDK states, as beneath every
///    request: they dial nothing.
/// 2. **The body is still read.** A body that is not the request is
///    said to be so, as on every leaf, before anything else.
/// 3. **The request is never put.** A served volume is a scope held
///    open on which the caller answers a filesystem: every read,
///    write and listing a mount makes, one channel each, for as long
///    as something is mounted from it. A command that prints lines and
///    exits has nothing to mount and nothing to answer with, so the
///    request is refused with an error that says so, and the daemon is
///    not dialled. It is here because every request of the daemon's
///    has a leaf; it is for a daemon mounting a volume of another's.
pub async fn run(arguments: Arguments, _target: &Target<'_>, emitter: &mut Emitter) -> Result<(), Error> {
    // Rule 1.
    match arguments.below {
        Some(Below::RequestSchema(_)) => {
            return request_schema::<request::Frame>("diverge run volumes fuse-serve", emitter).await;
        }
        Some(Below::ResponseSchema(_)) => {
            return response_schema::<response::Frame>("diverge run volumes fuse-serve", emitter).await;
        }
        None => {}
    }
    // Rule 2.
    let _request: request::Frame = body(arguments.body.as_deref())?;
    // Rule 3.
    Err(Error::Unsupported(
        "volumes fuse-serve holds a scope open and answers a filesystem's reads and writes on it, which a command cannot do; it is what one daemon asks of another to mount a volume from it",
    ))
}
