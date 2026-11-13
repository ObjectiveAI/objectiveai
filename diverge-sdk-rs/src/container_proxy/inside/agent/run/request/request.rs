//! What one run is asked.

/// Start one run: on chat messages, or on one structured decision.
/// The body is the [`Item`](crate::shared::containers::enqueue::request::Item)
/// an enqueue carries, tagged by `type`:
///
/// - `{"type":"chat","messages":[…]}` — one loop on these messages.
///   The arguments are not here: they were registered once, and they
///   never change. The messages are each loop's, because a container
///   runs loops one after another — each resuming the conversation the
///   last one left — and every one is asked something: one or more
///   messages, in the order their batches carried them, each under the
///   key its enqueue carried. The program joins them into ONE user
///   turn — the content of every message, in order — and yields each
///   message's user parts, under its own key, in order, as the
///   stream's first chunks, before anything else it says.
/// - `{"type":"structured_decisions","key":…,"content":[…],"questions":[…]}`
///   — one decision, alone. The program yields it back as the
///   stream's first chunk, one `user_structured_decisions` under its
///   key, says its answers in one `structured_decisions_response`, and
///   ends the stream. Nothing is enqueued onto such a run: the proxy
///   hands the program nothing more until the stream has ended.
///
/// A program that cannot take what it is asked — a block of a kind it
/// does not take, a structured decision where its image declares
/// none — answers a `4xx`, in its own words, and the proxy answers
/// everything the run was to take with them.
pub type Request = crate::shared::containers::enqueue::request::Item;
