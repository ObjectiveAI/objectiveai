//! The container's vault asks: the caller's vault, reached.

use std::sync::Arc;

use super::super::family::Runs;
use super::super::run::Run;
use super::one;
use crate::container_proxy::requests::execute::Ask;
use crate::container_proxy::vault;

/// One frame from the caller, or nothing, onto the ask's path.
pub(crate) async fn get<R: Runs>(run: Arc<Run>, ask: Ask) {
    let answer = one::ask::<R>(&run, &ask).await;
    let frame = answer.as_deref().and_then(|bytes| vault::get::response::Frame::decode(bytes).ok());
    let _ = vault::get::execute::execute(&run.client, ask.channel, frame.as_ref()).await;
}

/// One frame from the caller, or nothing, onto the ask's path.
pub(crate) async fn set<R: Runs>(run: Arc<Run>, ask: Ask) {
    let answer = one::ask::<R>(&run, &ask).await;
    let frame = answer.as_deref().and_then(|bytes| vault::set::response::Frame::decode(bytes).ok());
    let _ = vault::set::execute::execute(&run.client, ask.channel, frame.as_ref()).await;
}

/// One frame from the caller, or nothing, onto the ask's path.
pub(crate) async fn delete<R: Runs>(run: Arc<Run>, ask: Ask) {
    let answer = one::ask::<R>(&run, &ask).await;
    let frame = answer.as_deref().and_then(|bytes| vault::delete::response::Frame::decode(bytes).ok());
    let _ = vault::delete::execute::execute(&run.client, ask.channel, frame.as_ref()).await;
}

/// One frame from the caller, or nothing, onto the ask's path.
pub(crate) async fn lock<R: Runs>(run: Arc<Run>, ask: Ask) {
    let answer = one::ask::<R>(&run, &ask).await;
    let frame = answer.as_deref().and_then(|bytes| vault::lock::response::Frame::decode(bytes).ok());
    let _ = vault::lock::execute::execute(&run.client, ask.channel, frame.as_ref()).await;
}

/// One frame from the caller, or nothing, onto the ask's path.
pub(crate) async fn unlock<R: Runs>(run: Arc<Run>, ask: Ask) {
    let answer = one::ask::<R>(&run, &ask).await;
    let frame = answer.as_deref().and_then(|bytes| vault::unlock::response::Frame::decode(bytes).ok());
    let _ = vault::unlock::execute::execute(&run.client, ask.channel, frame.as_ref()).await;
}
