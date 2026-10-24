//! `diverge run volumes download`: downloading a file or a directory
//! out of a volume, as
//! [`download`](diverge_sdk::daemon::endpoints::volumes::download)
//! states it.

use std::path::PathBuf;

use bytes::Bytes;
use clap::Args;
use diverge_sdk::daemon::client::stream::{ExecuteStream, OpenError};
use diverge_sdk::daemon::endpoints::volumes::download::{
    client::execute, client::request, server::response,
};
use diverge_sdk::shared::error;
use diverge_sdk::wire::client::handle::Handle;
use serde::Serialize;

use crate::run::shape::download::Line;
use crate::run::shape::{Download, Downloaded};
use crate::run::{Below, Emitter, Error, Target, body, request_schema, response_schema, shape};

/// Downloading a file or a directory out of a volume.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
#[command(subcommand_negates_reqs = true, args_conflicts_with_subcommands = true)]
pub struct Arguments {
    /// The request itself: its body and the local path. Absent when a
    /// schema is asked for beneath, whose arguments these are not.
    #[command(flatten)]
    pub regular: Option<Regular>,
    /// What is beneath the request: `request-schema`, the JSON Schema
    /// of the body it takes, or `response-schema`, the JSON Schema of
    /// every line it prints — printed in place of putting any request.
    #[command(subcommand)]
    pub below: Option<Below>,
}

/// The request itself, as it is put: the body and the local path —
/// one group, so that a schema asked for beneath needs neither, and
/// a request without `--path` does not parse.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub struct Regular {
    /// What the request carries, as a JSON object: see
    /// [`request::Frame`]; `relay` among its members has another daemon
    /// serve it. Absent, `{}`.
    #[arg(value_name = "BODY")]
    pub body: Option<String>,
    /// The local file a file download is written to, or the local
    /// directory a directory download is written under, every parent
    /// made.
    #[arg(long = "path", value_name = "LOCAL")]
    pub local: PathBuf,
}

/// The request put, every file written under the local path, one
/// line per file written printed, and every other answer as it comes:
/// each a [`Line`].
pub async fn run(
    arguments: Arguments,
    target: &Target<'_>,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    match arguments.below {
        Some(Below::RequestSchema(_)) => {
            return request_schema::<request::Frame>("diverge run volumes download", emitter).await;
        }
        Some(Below::ResponseSchema(_)) => {
            return response_schema::<Line<Answer>>("diverge run volumes download", emitter).await;
        }
        None => {}
    }
    let Some(regular) = arguments.regular else {
        return Err(Error::NoRequest);
    };
    let request: request::Frame = body(regular.body.as_deref())?;
    let handle = target.dial().await?;
    shape::download(&handle, &request, &regular.local, emitter).await
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

/// A volume download's answer that is no chunk: the three every
/// download has, and `held` — a running container has the volume, or
/// another operation is on it. Printed as the daemon's own frame
/// would be.
#[derive(Debug, Clone, PartialEq, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Answer {
    /// No volume is the one named, or nothing is at the path.
    NotFound,
    /// A running container has the volume, or another download,
    /// upload or transfer is on it.
    Held,
    /// The account holds no grant allowing it.
    Forbidden,
    /// The daemon's own failure.
    Error(error::Error),
}

impl Downloaded for execute::Item {
    type Answer = Answer;

    fn chunk(self) -> Result<(Vec<String>, Bytes), Answer> {
        match self {
            execute::Item::Chunk { path, body } => Ok((path, body)),
            execute::Item::NotFound => Err(Answer::NotFound),
            execute::Item::Held => Err(Answer::Held),
            execute::Item::Forbidden => Err(Answer::Forbidden),
            execute::Item::Error(error) => Err(Answer::Error(error)),
        }
    }
}
