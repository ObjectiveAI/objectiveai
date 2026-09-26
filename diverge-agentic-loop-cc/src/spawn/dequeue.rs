//! The dequeue verb.

use std::collections::HashMap;

use diverge_container_proxy_sdk::agent::dequeue::Outcome;
use diverge_container_proxy_sdk::agent::enqueue::Fate;
use uuid::Uuid;

use crate::response;

use super::pending;
use super::replies;
use super::stdin;
use super::writer;

/// Withdraw everything still queued under a key.
///
/// BOTH locks are taken up front — joined, acquired in parallel, the
/// only place in the module that ever holds the two at once — and
/// the writer's FIFO queue position IS the withdrawal boundary: the
/// cancel "came in" the moment this joined that queue. Enqueues
/// ahead of it register before the snapshot and are withdrawn;
/// enqueues behind it register after and are never cancelled. Each
/// lock is held exactly as long as its job: the WRITER from the
/// pending-map snapshot through the cancel writes, dropped the
/// moment the write is through, so enqueues flow again during the
/// wait; the REPLIES through the reply reads, dropped when the last
/// reply is in. The writer's span is the correctness of the
/// snapshot — no enqueue can interleave between the look and the
/// cancels, so every message written before the cancels is
/// snapshotted and cancelled — those under the key; a message under
/// another key is not looked at. Messages enqueued during the reply
/// wait are after the withdrawal, and none of its business: they
/// write no control responses, so the replies read stay answers to
/// the cancels written. No timeout; the wait is as long as Claude
/// Code takes, and a run that ends under the wait closes the reply
/// channel, which resolves it too.
///
/// Each reply decides the fate of the message it answers for,
/// strictly: `cancelled: true` means the cancel reached it —
/// dequeued; `cancelled: false` means the queue no longer held it —
/// it was already taken, and its fate is delivered. A fate already
/// decided by someone faster, or one nobody is listening to, is
/// skipped — the reader races this same map on every replay echo.
pub async fn dequeue(key: &str) -> Outcome {
    // Joined, in parallel; the writer's fair queue makes this very
    // acquisition the withdrawal boundary.
    let (mut writer, mut replies) =
        tokio::join!(writer::WRITER.lock(), replies::REPLIES.lock());
    let Some(child_stdin) = writer.as_mut() else {
        return Outcome::Empty;
    };
    let Some(receiver) = replies.as_mut() else {
        // Unreachable in practice — the replies are set before the
        // writer — but a missing receiver reads as no run all the
        // same.
        return Outcome::Empty;
    };

    // The pending map's keys ARE the queue: entries leave as fates
    // are decided, so what remains is what a cancel can still speak
    // to. Complete under the writer lock — nothing can be inserted
    // while this holds it. Only the messages under the caller's key
    // are spoken to; the rest stay queued.
    let uuids: Vec<String> = pending::PENDING
        .iter()
        .filter(|entry| entry.value().key == key)
        .map(|entry| entry.key().clone())
        .collect();
    if uuids.is_empty() {
        return Outcome::Empty;
    }

    // One buffered write for all the withdrawals, each under a fresh
    // request id; the replies quote the ids back.
    let mut cancels: HashMap<String, String> = HashMap::new();
    let mut lines = String::new();
    for uuid in uuids {
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
        cancels.insert(request_id, uuid);
    }
    if stdin::write_lines(child_stdin, &lines).await.is_err() {
        // A broken stdin is the process dying: the run is over, and
        // a dead queue holds nothing. The pending fates stay for the
        // reader's end-of-stream to miss.
        *writer = None;
        *replies = None;
        return Outcome::Empty;
    }
    // The write is through: the writer's job here is done, and every
    // message a cancel could reach is in the snapshot. Enqueues flow
    // again from here — anything they queue is after the withdrawal.
    drop(writer);

    // Every cancel written gets its reply read, matched by request
    // id. A reply quoting an unknown id is stale — a prior dequeue
    // whose HTTP caller vanished mid-wait left it unread — and is
    // skipped, not counted.
    while !cancels.is_empty() {
        match receiver.recv().await {
            Some(reply) => match &reply.response {
                response::control::ControlResponseInner::Success {
                    request_id,
                    response,
                    ..
                } => {
                    let Some(uuid) = cancels.remove(request_id) else {
                        continue;
                    };
                    // The verdict rides the reply's `cancelled` flag,
                    // typed by the source as a plain boolean in the
                    // per-subtype payload map. Absent — schema drift
                    // the pinned image cannot produce — the fate is
                    // left for the end of stream to miss, visibly.
                    match response
                        .as_ref()
                        .and_then(|response| response.get("cancelled"))
                        .and_then(serde_json::Value::as_bool)
                    {
                        Some(true) => {
                            if let Some((_, pending)) =
                                pending::PENDING.remove(&uuid)
                            {
                                let _ = pending.fate.send(
                                    Fate::Dequeued,
                                );
                            }
                        }
                        Some(false) => {
                            if let Some((_, pending)) =
                                pending::PENDING.remove(&uuid)
                            {
                                let _ = pending.fate.send(
                                    Fate::Delivered,
                                );
                            }
                        }
                        None => {}
                    }
                }
                // The source cannot answer a cancel with an error;
                // if one arrives anyway, the reply is counted and
                // the fate left undecided, for the end of stream.
                response::control::ControlResponseInner::Error {
                    request_id,
                    ..
                } => {
                    cancels.remove(request_id);
                }
            },
            // The reader dropped the sender: the run is over, and a
            // dead queue holds nothing. The writer, released above,
            // is the reader's end-of-stream to clear — along with
            // missing the pending fates.
            None => {
                *replies = None;
                return Outcome::Empty;
            }
        }
    }

    // The last reply is in: the receiver's job here is done too.
    drop(replies);
    Outcome::Dequeued
}
