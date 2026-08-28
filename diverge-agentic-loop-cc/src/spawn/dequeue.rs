//! The dequeue verb.

use diverge_provider_sdk::agentic_loop_container;
use uuid::Uuid;

use crate::response;

use super::delivered;
use super::session;
use super::stdin;

/// Withdraw everything still queued.
///
/// The lock is held from the first look to the answer — across the
/// cancel writes AND the reply reads — and that hold is the whole
/// correctness: no enqueue can interleave, so the replies read are
/// answers to the cancels written, and receiving them means the
/// withdrawal resolved. No timeout; the wait is as long as Claude
/// Code takes, and a run that ends under the wait closes the reply
/// channel, which resolves it too.
pub async fn dequeue() -> agentic_loop_container::dequeue::Response {
    let mut session = session::SESSION.lock().await;
    let Some(inner) = session.as_mut() else {
        return agentic_loop_container::dequeue::Response::Empty {
            r#type: Default::default(),
        };
    };

    // Pre-clean: a message known delivered is not in the queue, and
    // writing a cancel for it would be asking about the past.
    inner.queued.retain(|uuid| !delivered::DELIVERED.contains(uuid));
    if inner.queued.is_empty() {
        return agentic_loop_container::dequeue::Response::Empty {
            r#type: Default::default(),
        };
    }

    // One buffered write for all the withdrawals, each under a fresh
    // request id; the replies quote the ids back.
    let mut request_ids = std::collections::HashSet::new();
    let mut lines = String::new();
    for uuid in &inner.queued {
        let request_id = Uuid::new_v4().to_string();
        lines.push_str(
            &serde_json::to_string(&stdin::ControlRequest {
                r#type: Default::default(),
                request_id: request_id.clone(),
                request: stdin::CancelAsyncMessage {
                    subtype: Default::default(),
                    message_uuid: uuid.clone(),
                },
            })
            .expect("a stdin line is plain structs and serializes"),
        );
        lines.push('\n');
        request_ids.insert(request_id);
    }
    if stdin::write_lines(&mut inner.stdin, &lines).await.is_err() {
        // A broken stdin is the process dying: the session is over,
        // and a dead queue holds nothing.
        *session = None;
        return agentic_loop_container::dequeue::Response::Empty {
            r#type: Default::default(),
        };
    }

    // Every cancel written gets its reply read, matched by request
    // id. A reply quoting an unknown id is stale — a prior dequeue
    // whose HTTP caller vanished mid-wait left it unread — and is
    // skipped, not counted.
    while !request_ids.is_empty() {
        match inner.replies.recv().await {
            Some(reply) => {
                let (response::control::ControlResponseInner::Success {
                    request_id,
                    ..
                }
                | response::control::ControlResponseInner::Error {
                    request_id,
                    ..
                }) = &reply.response;
                request_ids.remove(request_id);
            }
            // The reader dropped the sender: the run is over, and a
            // dead queue holds nothing.
            None => {
                *session = None;
                return agentic_loop_container::dequeue::Response::Empty {
                    r#type: Default::default(),
                };
            }
        }
    }

    inner.queued.clear();
    agentic_loop_container::dequeue::Response::Dequeued {
        r#type: Default::default(),
    }
}
