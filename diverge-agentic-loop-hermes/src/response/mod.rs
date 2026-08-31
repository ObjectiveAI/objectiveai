//! The gateway's run-event vocabulary, whole.
//!
//! Everything `GET /v1/runs/{run_id}/events` can send, as types —
//! verified against the pinned Hermes source, recorded in
//! `reports/14.md`. Deserialize-only: this is the direction the
//! harness reads, and it never writes these.
//!
//! The stream's contract, in brief:
//!
//! - Every frame is `data: {json}\n\n` — the gateway never writes
//!   an SSE `event:` line; the payload's own `event` key is the
//!   whole discrimination, which is what [`Event`] reads.
//! - Two SSE COMMENTS ride beside the events and must be skipped:
//!   `: keepalive` every 30 idle seconds, and `: stream closed` as
//!   the termination sentinel. The sentinel is a comment a strict
//!   SSE reader never surfaces — and a mid-stream failure closes
//!   the socket with no sentinel at all — so EOF must always end
//!   the stream, terminal event seen or not.
//! - The queue behind the stream is unbounded and fills from the
//!   run's POST onward, so a late subscriber receives the whole
//!   history in order. Consumption is destructive and the queue is
//!   ONE subscriber's: a second concurrent reader steals frames.
//!   Unsubscribed queues expire after 300 seconds.
//! - There is no `run.started`, no message framing around the
//!   deltas, no `done` event, and no resume.

mod approval_request;
mod approval_responded;
mod event;
mod message_delta;
mod reasoning_available;
mod run_cancelled;
mod run_completed;
mod run_failed;
mod run_steered;
mod subagent_complete;
mod subagent_start;
mod tool_completed;
mod tool_started;

pub use approval_request::*;
pub use approval_responded::*;
pub use event::*;
pub use message_delta::*;
pub use reasoning_available::*;
pub use run_cancelled::*;
pub use run_completed::*;
pub use run_failed::*;
pub use run_steered::*;
pub use subagent_complete::*;
pub use subagent_start::*;
pub use tool_completed::*;
pub use tool_started::*;
