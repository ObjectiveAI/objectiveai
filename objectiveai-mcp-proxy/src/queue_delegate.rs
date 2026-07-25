//! In-process delegate the embedder (the API) plugs in at
//! [`crate::setup`] time so the proxy can surface client-notification
//! content blocks on tool responses without HTTP round-trips.
//!
//! The proxy is **fully naive** about ban lists, content-row ids, and
//! confirmation state. It hands the trait the per-session agent
//! arguments on each call and either gets back a [`QueueRead`] (token +
//! blocks) to splice into the next
//! tool response, or `None` for "nothing to surface right now"
//! (including errors — the proxy never sees a `Result`).
//!
//! The `token` is opaque to the proxy: it gets embedded in the
//! wrapper prefix the agent sees (via
//! [`objectiveai_sdk::mcp::queue_notification::format_prefix`]),
//! round-trips through the agent's tool-message text, and lands
//! back at the API's run-loop, which calls back into its own
//! `ApiQueueDelegate::confirm(token)` to finalize delivery. Tokens
//! the run-loop never sees stay in the delegate's "pending" map
//! until the loop unregisters, at which point those ids re-deliver
//! on the next loop. That's how we avoid claiming delivery for
//! content the agent didn't actually consume.
//!
//! The proxy calls the delegate **sequentially after the tool call
//! succeeds**. Running the read in parallel with the tool call would
//! advance the delegate's pending map even when the call ends up
//! returning a JSON-RPC error (ToolNotFound / Upstream) — those rows
//! would never surface in any tool-message text and would never get
//! confirmed (correct behavior), but the wasted speculative state
//! makes pending bloat unnecessarily. Sequential keeps things tidy.

use std::future::Future;
use std::pin::Pin;

use indexmap::IndexMap;
use objectiveai_sdk::mcp::tool::ContentBlock;

/// Successful return shape: a per-row block list plus a
/// confirmation token the delegate has reserved for this batch.
/// The proxy embeds `token` in the wrapper prefix it prepends to
/// the tool response; it does nothing else with the token.
pub struct QueueRead {
    /// Opaque confirmation token. The agent's tool-message text
    /// will carry it inline; `run_agent_loop` regex-extracts it
    /// and calls back to confirm delivery.
    pub token: String,
    /// One inner `Vec<ContentBlock>` per consumed `message_queue`
    /// row, oldest-first.
    pub blocks: Vec<Vec<ContentBlock>>,
}

/// Embedder-provided message-queue read path. `None` ⇒ no delegate
/// installed at proxy boot (CLI standalone case); the proxy's
/// tool-call dispatcher short-circuits without invoking anything.
///
/// The trait method returns a boxed-pinned `Future + Send` rather
/// than using `async_trait`: no proc-macro dep, and the trait
/// stays `dyn`-safe so the proxy can hold it as
/// `Arc<dyn QueueDelegate>`. Implementations can still write the
/// body as an `async move { ... }` block wrapped in `Box::pin`.
pub trait QueueDelegate: Send + Sync {
    /// Read pending content blocks for one session. Returns
    /// `Some(QueueRead { token, blocks })` on success, `None` on
    /// error / empty / "nothing right now."
    ///
    /// `identity` is the proxy's per-session transient
    /// header map (the agent-routing keys like
    /// `X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY` and
    /// `X-OBJECTIVEAI-RESPONSE-ID`); the delegate parses out whatever
    /// it needs to look up the right per-loop state.
    fn read_pending_blocks<'a>(
        &'a self,
        identity: &'a IndexMap<String, String>,
    ) -> Pin<Box<dyn Future<Output = Option<QueueRead>> + Send + 'a>>;
}
