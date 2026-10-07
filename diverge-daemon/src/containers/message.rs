//! A message to an agent: the one way into it.

use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::shared::containers::enqueue;
use diverge_sdk::shared::error::Error;
use rmcp::model::ContentBlock;

use super::AgentRun;

/// What became of a message.
pub enum Fate {
    /// The loop took it.
    Delivered,
    /// It was taken back before the loop took it.
    Cancelled,
    /// The container refused it, or the run is gone.
    Error(Error),
}

/// Enqueue the message under a key the daemon mints, with who sent
/// it kept so its user parts are logged as theirs, and wait for its
/// fate — however long the loop takes to reach a seam. The key is
/// what a cancel names.
pub async fn enqueue(run: &AgentRun, content: Vec<ContentBlock>, sender: Creator) -> Fate {
    let key = uuid::Uuid::new_v4().to_string();
    run.messages.lock().await.insert(key.clone(), sender);
    run.touch();
    let fate = run.handle.enqueue(key.clone(), content).await;
    let fate = match fate {
        Ok(enqueue::response::Frame::Delivered) => Fate::Delivered,
        Ok(enqueue::response::Frame::Dequeued) => Fate::Cancelled,
        Ok(enqueue::response::Frame::Error(error)) => Fate::Error(error),
        Err(error) => Fate::Error(Error(serde_json::json!({
            "kind": "daemon",
            "error": format!("the message could not be enqueued: {error}"),
        }))),
    };
    if !matches!(fate, Fate::Delivered) {
        run.messages.lock().await.remove(&key);
    }
    fate
}

/// Enqueue the message, and hand back the key before the fate is
/// known, so that a cancel can name it: the fate is the future.
pub async fn enqueue_keyed(run: &AgentRun, content: Vec<ContentBlock>, sender: Creator) -> (String, impl std::future::Future<Output = Fate> + '_) {
    let key = uuid::Uuid::new_v4().to_string();
    run.messages.lock().await.insert(key.clone(), sender);
    run.touch();
    let named = key.clone();
    let fate = async move {
        let fate = match run.handle.enqueue(named.clone(), content).await {
            Ok(enqueue::response::Frame::Delivered) => Fate::Delivered,
            Ok(enqueue::response::Frame::Dequeued) => Fate::Cancelled,
            Ok(enqueue::response::Frame::Error(error)) => Fate::Error(error),
            Err(error) => Fate::Error(Error(serde_json::json!({
                "kind": "daemon",
                "error": format!("the message could not be enqueued: {error}"),
            }))),
        };
        if !matches!(fate, Fate::Delivered) {
            run.messages.lock().await.remove(&named);
        }
        fate
    };
    (key, fate)
}

/// Take the message under the key back, if the loop has not taken
/// it: its enqueue answers `Dequeued`.
pub async fn cancel(run: &AgentRun, key: &str) {
    let _ = run.handle.dequeue(key.to_string()).await;
}
