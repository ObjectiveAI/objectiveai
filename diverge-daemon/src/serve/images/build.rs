//! Building an image on a provider.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::images::build::client::request;
use diverge_sdk::daemon::endpoints::images::build::server::response::{Built, Exists, Frame};
use diverge_sdk::daemon::grant::images::Action;
use diverge_sdk::daemon::grant::volumes::Over;
use diverge_sdk::daemon::reference;
use diverge_sdk::provider::endpoints::images::build::client::execute::{ExecuteStreamError, execute};
use diverge_sdk::provider::endpoints::images::build::client::request::{Context, Frame as Ask};
use diverge_sdk::provider::endpoints::images::build::server::response::Frame as Said;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use futures_util::StreamExt as _;

use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::volumes::{Located, locate};
use crate::serve::{inner, reply};
use crate::store;
use crate::volumes;

/// Answer the build — its output as it comes, then how it ended — and
/// finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) {
    let answer = match serve(&scope, frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// The answer that ends the build, every line of its output sent on
/// the scope before it. The rules:
///
/// 1. `Forbidden` with no `build` grant over images.
/// 2. `NotFound` for a provider the daemon keeps no record of.
/// 3. With a context: `Forbidden` with no `mount` grant over volumes
///    at all; `NotFound` for a volume the provider does not list;
///    `Error` for a provider that could not be asked; `Forbidden` for
///    a volume the grants do not reach.
/// 4. Then the provider is asked, as the daemon itself, so the image
///    is built into the daemon's Store there. `Error` for a provider
///    not connected now, or a build that would not open.
/// 5. Each line of output is relayed as it comes. The provider's
///    `Built`, `Exists` and its three refusals are relayed as they
///    are; its error is the `Error`, in its words; an exchange that
///    broke, or ended with no result, is the `Error`, in the daemon's.
async fn serve(scope: &ScopeHandle, frame: request::Frame, who: Who, daemon: &Arc<Daemon>) -> Result<Frame, store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        return Ok(Frame::Forbidden);
    };
    // Rule 1.
    if !judge::images::holds(&standing, Action::Build) {
        return Ok(Frame::Forbidden);
    }
    // Rule 2.
    if !inner::on_record(daemon, &frame.provider) {
        return Ok(Frame::NotFound);
    }
    // Rule 3.
    if let Some(context) = &frame.context {
        if !judge::volumes::holds(&standing, Over::Mount) {
            return Ok(Frame::Forbidden);
        }
        let volume = reference::Volume {
            provider: frame.provider.clone(),
            name: context.volume.clone(),
        };
        let listed = match locate(&mut conn, daemon, &volume).await? {
            Located::Volume(listed) => listed,
            Located::None => return Ok(Frame::NotFound),
            Located::Failed(error) => return Ok(Frame::Error(reply::failure(&error))),
        };
        if !judge::volumes::over(&standing, Over::Mount, &listed) {
            return Ok(Frame::Forbidden);
        }
    }
    drop(conn);
    scope.allowed().await;
    // Rule 4.
    let handle = match volumes::handle(daemon, &frame.provider).await {
        Ok(handle) => handle,
        Err(fail) => return Ok(Frame::Error(fail.error())),
    };
    let ask = Ask {
        description: frame.description,
        containerfile: frame.containerfile,
        context: frame.context.map(|context| Context {
            volume_name: context.volume,
            volume_relative_path: context.path,
        }),
    };
    let mut said = match execute(&handle, &ask).await {
        Ok(said) => said,
        Err(error) => return Ok(Frame::Error(reply::failure(&error))),
    };
    // Rule 5.
    while let Some(next) = said.next().await {
        match next {
            Ok(Said::Output(line)) => reply::reply(scope, &Frame::Output(line)).await,
            Ok(Said::Built(built)) => {
                return Ok(Frame::Built(Built {
                    digest: built.digest,
                    bytes: built.bytes,
                }));
            }
            Ok(Said::Exists(exists)) => return Ok(Frame::Exists(Exists { digest: exists.digest })),
            Ok(Said::InsufficientCapacity) | Err(ExecuteStreamError::InsufficientCapacity) => return Ok(Frame::InsufficientCapacity),
            Ok(Said::ContextHeld) | Err(ExecuteStreamError::ContextHeld) => return Ok(Frame::ContextHeld),
            Ok(Said::Unsupported) | Err(ExecuteStreamError::Unsupported) => return Ok(Frame::Unsupported),
            Ok(Said::Error(error)) | Err(ExecuteStreamError::Refused(error)) => return Ok(Frame::Error(error)),
            Err(error) => return Ok(Frame::Error(reply::failure(&error))),
        }
    }
    Ok(Frame::Error(reply::failure(&"the provider ended the build with no result")))
}
