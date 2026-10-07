//! Creating a volume.

use diverge_sdk::daemon::endpoints::volumes::create::client::request;
use diverge_sdk::daemon::endpoints::volumes::create::server::response::Frame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

use crate::daemon::Daemon;
use crate::judge::{self, Standing, Who};
use crate::serve::{inner, reply};
use crate::store;
use crate::volumes::{self, Made};

/// Answer the create and finish the scope.
pub async fn handle(scope: ScopeHandle, frame: request::Frame, who: Who, daemon: &Daemon) {
    let answer = match serve(frame, who, daemon).await {
        Ok(answer) => answer,
        Err(error) => Frame::Error(reply::failure(&error)),
    };
    reply::reply(&scope, &answer).await;
    scope.send_response_finish().await;
}

/// `Forbidden` without the `create` make; `NoProvider` for a provider
/// not on record; `Error` for one not connected; `Exists` for a name
/// the provider lists already; `InsufficientCapacity` as the provider
/// says; else `Created`.
async fn serve(frame: request::Frame, who: Who, daemon: &Daemon) -> Result<Frame, store::Error> {
    let mut conn = daemon.store.acquire().await?;
    let Some(standing) = Standing::of(&mut conn, who).await? else {
        return Ok(Frame::Forbidden);
    };
    if !judge::volumes::create(&standing) {
        return Ok(Frame::Forbidden);
    }
    if !inner::on_record(&mut conn, &frame.provider).await? {
        return Ok(Frame::NoProvider);
    }
    drop(conn);
    let listed = match volumes::list(daemon, &frame.provider).await {
        Ok(listed) => listed,
        Err(fail) => return Ok(Frame::Error(reply::failure(&fail))),
    };
    if listed.iter().any(|volume| volume.name == frame.name) {
        return Ok(Frame::Exists);
    }
    Ok(match volumes::create(daemon, &frame.provider, frame.name, frame.bytes, frame.mode).await {
        Ok(Made::Created) => Frame::Created,
        Ok(Made::InsufficientCapacity) => Frame::InsufficientCapacity,
        Err(fail) => Frame::Error(reply::failure(&fail)),
    })
}
