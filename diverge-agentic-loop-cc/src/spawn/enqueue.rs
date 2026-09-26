//! The enqueue verb.

use diverge_sdk::container_proxy::inside::agent::enqueue::Fate;
use rmcp::model::ContentBlock;
use uuid::Uuid;

use super::pending;
use super::stdin;
use super::writer;

/// Queue a message for the running session, and answer its fate —
/// STRICT: the response leaves when the fate is known, not when the
/// write lands.
///
/// The write and the fate's registration happen under the writer
/// lock, together — so a dequeue, which holds that lock throughout,
/// always sees every message written before it in the pending map.
/// Then the lock drops and the wait begins: whoever decides the fate
/// — the reader on a replay echo, a dequeue's cancel, the end of
/// stream — sends it here. No run to write to, a failed write, or a
/// fate wire dying undecided all answer missed.
pub async fn enqueue(key: String, content: Vec<ContentBlock>, blocks: Vec<stdin::Block>) -> Fate {
    let (fate, receiver) = tokio::sync::oneshot::channel();
    let uuid = Uuid::new_v4().to_string();
    {
        // The writer, before anything: its FIFO queue is the
        // withdrawal boundary. A dequeue queued behind this enqueue
        // snapshots after the registration below and withdraws it; a
        // dequeue queued ahead has already excluded it.
        let mut writer = writer::WRITER.lock().await;
        let Some(child_stdin) = writer.as_mut() else {
            return Fate::Missed;
        };
        match stdin::write_lines(
            child_stdin,
            &stdin::user_message_line(blocks, uuid.clone()),
        )
        .await
        {
            Ok(()) => {
                pending::PENDING.insert(uuid.clone(), pending::Pending { key, content, fate });
            }
            // A broken stdin is the process dying: the run is over.
            Err(_) => {
                *writer = None;
                return Fate::Missed;
            }
        }
    }
    let response = receiver.await.unwrap_or(
        // The wire died undecided — the run's machinery is gone, and
        // a message nobody will take is missed.
        Fate::Missed,
    );
    // Whoever decided the fate already removed the entry; this tidies
    // up the cases nobody else covers, and is a no-op otherwise.
    pending::PENDING.remove(&uuid);
    response
}
