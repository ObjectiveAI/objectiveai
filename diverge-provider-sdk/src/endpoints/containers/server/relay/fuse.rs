//! The container's FUSE asks: the files the caller serves live.

use std::sync::Arc;

use super::super::family::Runs;
use super::super::run::Run;
use super::one;
use crate::container_proxy::fuse;
use crate::container_proxy::requests::execute::Ask;

/// One frame from the caller, or nothing, onto the ask's path.
pub(crate) async fn read<R: Runs>(run: Arc<Run>, ask: Ask) {
    let answer = one::ask::<R>(&run, &ask).await;
    let frame = answer.as_deref().and_then(|bytes| fuse::read::response::Frame::decode(bytes).ok());
    let _ = fuse::read::execute::execute(&run.client, ask.channel, frame.as_ref()).await;
}

/// One frame from the caller, or nothing, onto the ask's path.
pub(crate) async fn write<R: Runs>(run: Arc<Run>, ask: Ask) {
    let answer = one::ask::<R>(&run, &ask).await;
    let frame = answer.as_deref().and_then(|bytes| fuse::write::response::Frame::decode(bytes).ok());
    let _ = fuse::write::execute::execute(&run.client, ask.channel, frame.as_ref()).await;
}

/// One frame from the caller, or nothing, onto the ask's path.
pub(crate) async fn list<R: Runs>(run: Arc<Run>, ask: Ask) {
    let answer = one::ask::<R>(&run, &ask).await;
    let frame = answer.as_deref().and_then(|bytes| fuse::list::response::Frame::decode(bytes).ok());
    let _ = fuse::list::execute::execute(&run.client, ask.channel, frame.as_ref()).await;
}

/// One frame from the caller, or nothing, onto the ask's path.
pub(crate) async fn remove<R: Runs>(run: Arc<Run>, ask: Ask) {
    let answer = one::ask::<R>(&run, &ask).await;
    let frame = answer.as_deref().and_then(|bytes| fuse::remove::response::Frame::decode(bytes).ok());
    let _ = fuse::remove::execute::execute(&run.client, ask.channel, frame.as_ref()).await;
}

/// One frame from the caller, or nothing, onto the ask's path.
pub(crate) async fn rename<R: Runs>(run: Arc<Run>, ask: Ask) {
    let answer = one::ask::<R>(&run, &ask).await;
    let frame = answer.as_deref().and_then(|bytes| fuse::rename::response::Frame::decode(bytes).ok());
    let _ = fuse::rename::execute::execute(&run.client, ask.channel, frame.as_ref()).await;
}

/// One frame from the caller, or nothing, onto the ask's path.
pub(crate) async fn mkdir<R: Runs>(run: Arc<Run>, ask: Ask) {
    let answer = one::ask::<R>(&run, &ask).await;
    let frame = answer.as_deref().and_then(|bytes| fuse::mkdir::response::Frame::decode(bytes).ok());
    let _ = fuse::mkdir::execute::execute(&run.client, ask.channel, frame.as_ref()).await;
}

/// One frame from the caller, or nothing, onto the ask's path.
pub(crate) async fn stat<R: Runs>(run: Arc<Run>, ask: Ask) {
    let answer = one::ask::<R>(&run, &ask).await;
    let frame = answer.as_deref().and_then(|bytes| fuse::stat::response::Frame::decode(bytes).ok());
    let _ = fuse::stat::execute::execute(&run.client, ask.channel, frame.as_ref()).await;
}
