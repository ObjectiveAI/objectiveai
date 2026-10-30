//! A standalone tool's log: every exchange the daemon routed into it, read.
//!
//! What the log holds is [`tool_log`](crate::daemon::tool_log)'s: the
//! calls and reads the tool was asked, each with who asked, their
//! answers, its stretches of use, its notifications and the daemon's
//! own errors over it.
//!
//! One scope reads it, as an agent's log is read
//! ([`agents::logs`](crate::daemon::endpoints::agents::logs)): a client
//! names the tool and says what it wants — a span of indexes, a span of
//! time, one kind of item, how many items at most, and whether to
//! watch, every one optional. The daemon runs the filter over the log
//! as it stands, oldest first, one response per item, and finishes; or,
//! watching, goes on over each item as it lands until the filter can
//! never match again, the count is met, the client cancels, or
//! the tool is deleted. A tool that does not exist is the scope's error.
//!
//! Split by who SENDS, as everywhere else: the question and the cancel
//! are in [`client`], the answer in [`server`].

pub mod client;
pub mod server;
