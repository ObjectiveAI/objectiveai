//! `diverge run standalone-tools frontend`: one HTTP request to a standalone tool's
//! frontend, its body this command's stdin, as
//! [`frontend`](diverge_sdk::daemon::endpoints::standalone_tools::frontend) states it.

use clap::Args;
use diverge_sdk::daemon::endpoints::standalone_tools::frontend::{client::execute, client::request, server::response};

use crate::run::shape::frontend::Said;
use crate::run::{Below, Emitter, Error, Target, body, request_schema, response_schema, shape};

/// One HTTP request to a standalone tool's frontend.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
#[command(subcommand_negates_reqs = true, args_conflicts_with_subcommands = true)]
pub struct Arguments {
    /// What the request carries, as a JSON object: see
    /// [`request::Frame`] — whose frontend, the `method`, the `path`
    /// beneath `/frontend`, and the `headers`; `relay` among its members
    /// has another daemon serve it.
    #[arg(value_name = "BODY")]
    pub body: Option<String>,
    /// What is beneath the request: `request-schema`, the JSON Schema
    /// of the body it takes, or `response-schema`, the JSON Schema of
    /// every answer — printed in place of putting any request.
    #[command(subcommand)]
    pub below: Option<Below>,
}

/// The request put, this command's stdin its body, the response's head
/// on stderr and its body on stdout (see
/// [`shape::frontend`](crate::run::shape::frontend)).
pub async fn run(arguments: Arguments, target: &Target<'_>, emitter: &mut Emitter) -> Result<(), Error> {
    match arguments.below {
        Some(Below::RequestSchema(_)) => {
            return request_schema::<request::Frame>("diverge run standalone-tools frontend", emitter).await;
        }
        Some(Below::ResponseSchema(_)) => {
            return response_schema::<response::Frame>("diverge run standalone-tools frontend", emitter).await;
        }
        None => {}
    }
    let request: request::Frame = body(arguments.body.as_deref())?;
    let handle = target.dial().await?;
    let (answers, frontend) = execute::execute(&handle, &request).await.map_err(shape::exchange::open)?;
    let writer = frontend.clone();
    let feed = move |bytes: Vec<u8>| {
        let frontend = writer.clone();
        async move { frontend.body(&bytes).await.is_ok() }
    };
    let end = move || async move {
        let _ = frontend.end().await;
    };
    shape::frontend::frontend(answers, feed, end, said).await
}

/// What an answer says.
fn said(frame: &response::Frame) -> Said {
    match frame {
        response::Frame::Head(_) => Said::Head,
        response::Frame::Body(bytes) => Said::Body(bytes.clone()),
        response::Frame::NotFound | response::Frame::Forbidden | response::Frame::NoFrontend | response::Frame::Error(_) => Said::Refused,
    }
}
