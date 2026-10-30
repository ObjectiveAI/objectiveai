//! A log, read: an agent's or a tool's, the subject named.
//!
//! The daemon keeps a log per agent — [`agent_log`](crate::daemon::agent_log),
//! what its run streamed — and per tool —
//! [`tool_log`](crate::daemon::tool_log), every call and read it routed
//! into the tool. One scope reads any of them. A client names the
//! subject and says what it wants: a span of indexes, a span of time,
//! one kind of item, how many items at most, and whether to watch —
//! every one optional, and a request that names only the subject is
//! the whole log. The daemon runs the filter over the log as it stands,
//! oldest first, one response per item, and finishes; or, watching,
//! goes on to run the same filter over each item as it lands, and stays
//! open until the filter can never match again — the log has reached
//! the request's last index, or the clock its last time — or the count
//! is met, or the client cancels, or the subject is deleted. A filter
//! with no last index, no last time and no count can always match
//! again, and a watch on it never ends on its own. A subject that does
//! not exist is the scope's error.
//!
//! Each item comes wrapped in its subject's kind — an
//! [`Entry`](server::response::Entry) — since the two logs' vocabularies
//! share some names and not their shapes. Who may read which log is
//! the subject's own family's `logs` grant.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question and the cancel are in [`client`] — and the daemon answers,
//! so the answer is in [`server`].

pub mod client;
pub mod server;
