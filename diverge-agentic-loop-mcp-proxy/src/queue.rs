//! The proxy's message queue: steering folded onto the tool seam.
//!
//! A harness whose upstream cannot take a message mid-turn (Hermes
//! over ACP has no steer verb; only Claude Code's own wire does)
//! still owns one seam every agent crosses: its tool calls transit
//! THIS proxy. So the queue lives here — `POST /enqueue` puts a
//! message in, and the next tool response relayed back to the agent
//! carries everything pending, folded in front of the tool's own
//! content as one `<system-reminder>` section, exactly the fold the
//! openrouter harness performs locally.
//!
//! # The correlation is the content's own hash
//!
//! The enqueue's answer names WHERE the message landed: the SHA-256
//! of the folded response's raw text. The harness watches tool
//! responses on its own wire, hashes their text the same way, and
//! the match is the delivery position — no token invented, no id
//! smuggled through renderers that would launder it. See
//! [`fold`] for the canonical hash definition.
//!
//! # Every enqueue is answered
//!
//! Attached by the next fold, or dequeued by a clearing — one of
//! the two, always, with no timeout: the queue and its lock live as
//! long as the proxy, and the proxy as long as the run.
//!
//! # Order is kept
//!
//! The queue is a vector, so enqueue order is delivery order: one
//! fold takes everything pending and joins the prompts in exactly
//! the order they arrived.

use rmcp::model::{CallToolResult, ContentBlock};
use serde::Serialize;
use sha2::{Digest as _, Sha256};
use tokio::sync::{Mutex, oneshot};

/// The queue.
pub static QUEUE: Queue = Queue {
    pending: Mutex::const_new(Vec::new()),
};

/// One message waiting, and the wire its answer goes back on.
struct Pending {
    /// The message's text.
    prompt: String,
    /// Where the answer goes. A receiver that hung up stopped
    /// listening; the delivery happens regardless.
    reply: oneshot::Sender<Enqueued>,
}

/// What an enqueue is eventually told.
///
/// Untagged, discriminated by payload, the house way for JSON
/// unions: each variant's one field is a name no other variant
/// carries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Enqueued {
    /// Folded onto a tool response: the SHA-256 (lowercase hex) of
    /// that response's raw text, the caller's correlation key.
    Attached {
        /// The hash of the response the message rides in.
        attached: String,
    },
    /// Withdrawn by a dequeue before any tool response came.
    Dequeued {
        /// Always `true` — the variant's marker.
        dequeued: bool,
    },
}

/// The queue itself: a [`Mutex`] around the pending messages.
/// Nothing awaits while holding the lock; the waiting — the whole
/// substance of an enqueue — happens on the [`oneshot`] outside it.
pub struct Queue {
    pending: Mutex<Vec<Pending>>,
}

impl Queue {
    /// Put a message in, and get the wire its answer will arrive on.
    pub async fn enqueue(
        &self,
        prompt: String,
    ) -> oneshot::Receiver<Enqueued> {
        let (reply, receiver) = oneshot::channel();
        self.pending.lock().await.push(Pending { prompt, reply });
        receiver
    }

    /// Withdraw everything pending, answering each message dequeued.
    /// Returns how many were withdrawn.
    pub async fn dequeue(&self) -> usize {
        let taken = std::mem::take(&mut *self.pending.lock().await);
        let count = taken.len();
        for pending in taken {
            let _ = pending
                .reply
                .send(Enqueued::Dequeued { dequeued: true });
        }
        count
    }

    /// The seam: fold everything pending onto this tool response,
    /// and tell every folded message where it landed.
    ///
    /// The pending prompts — in enqueue order, joined by blank
    /// lines, wrapped in one `<system-reminder>` section — are
    /// PREPENDED as a text block at position 0: the position Claude
    /// Code's own folding uses, and the end a tail-truncating
    /// renderer keeps. A response with no pending messages is left
    /// untouched, byte for byte.
    ///
    /// # The canonical hash
    ///
    /// SHA-256, lowercase hex, over the UTF-8 bytes of every text
    /// content block's text in the FOLDED result, concatenated in
    /// arrival order with no separators. A harness recomputes this
    /// from the tool response text its own wire shows it; matching
    /// is the delivery position.
    pub async fn fold(&self, result: &mut CallToolResult) {
        let taken = std::mem::take(&mut *self.pending.lock().await);
        if taken.is_empty() {
            return;
        }

        let section = format!(
            "<system-reminder>\nThe user sent a new message while you were working:\n{}\n</system-reminder>\n\n",
            taken
                .iter()
                .map(|pending| pending.prompt.as_str())
                .collect::<Vec<_>>()
                .join("\n\n"),
        );
        result
            .content
            .insert(0, ContentBlock::text(section));

        let mut hasher = Sha256::new();
        for block in &result.content {
            if let Some(text) = block.as_text() {
                hasher.update(text.text.as_bytes());
            }
        }
        let attached = format!("{:x}", hasher.finalize());

        for pending in taken {
            let _ = pending.reply.send(Enqueued::Attached {
                attached: attached.clone(),
            });
        }
    }
}
