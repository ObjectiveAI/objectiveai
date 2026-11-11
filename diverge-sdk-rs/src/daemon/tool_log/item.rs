//! What one entry of a tool's log holds.

use serde::{Deserialize, Serialize};

use super::{Call, CallError, CallInterrupted, CallResult, Error, Notification, Read, ReadError, ReadInterrupted, ReadResult};
use crate::daemon::command_log::Command;
use crate::daemon::lifecycle;

/// One item of a tool's log, told apart by `type`.
///
/// The rules:
///
/// 0. **The run's life.** [`Deployed`](Self::Deployed),
///    [`Active`](Self::Active), [`Inactive`](Self::Inactive) and
///    [`Undeployed`](Self::Undeployed) are kept as
///    [`lifecycle`](crate::daemon::lifecycle) states, in the same JSON
///    an agent's log keeps them in: every ask falls inside a run's
///    `deployed` … `undeployed`.
/// 1. An ask — a [`Call`] or a [`Read`] — is kept when the daemon
///    routes it to a deployed run, before the tool is asked; its
///    answer — a result or an error — when the tool answers, naming
///    the ask by its `logs_index`. An ask the run went down under is
///    answered [`CallInterrupted`] or [`ReadInterrupted`], just before
///    the run's `inactive` and `undeployed`. Every ask is answered
///    exactly once in the log.
/// 2. [`Active`](Self::Active) is kept just before an ask while no
///    other ask is in flight, and [`Inactive`](Self::Inactive) just
///    after the answer that leaves none in flight, so the asks between
///    the two are exactly those of one stretch of use.
/// 3. A [`Notification`] is kept once per notification the tool sends,
///    naming the caller whose session it was sent on; it is no ask's,
///    and moves nothing in flight.
/// 4. An [`Error`] is the daemon's own, met keeping the tool — never an
///    ask's answer.
/// 5. A [`Command`] is a request the tool's own container put to the
///    daemon, kept as [`command_log`](crate::daemon::command_log) states;
///    it moves nothing in flight.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Item {
    /// The tool's container came up on a provider: `deployed`.
    Deployed(lifecycle::Deployed),
    /// An ask came in while none was in flight: `active`.
    Active,
    /// The last ask in flight was answered: `inactive`.
    Inactive,
    /// The tool's container went down, and why when unexpectedly:
    /// `undeployed`.
    Undeployed(lifecycle::Undeployed),
    /// A tool call: `call`.
    Call(Call),
    /// A tool call's result: `call_result`.
    CallResult(CallResult),
    /// A tool call that failed: `call_error`.
    CallError(CallError),
    /// A tool call the run went down under: `call_interrupted`.
    CallInterrupted(CallInterrupted),
    /// A resource read: `read`.
    Read(Read),
    /// A resource read's result: `read_result`.
    ReadResult(ReadResult),
    /// A resource read that failed: `read_error`.
    ReadError(ReadError),
    /// A resource read the run went down under: `read_interrupted`.
    ReadInterrupted(ReadInterrupted),
    /// A notification the tool sent: `notification`.
    Notification(Notification),
    /// An error the daemon met keeping the tool: `error`.
    Error(Error),
    /// A request the tool put to the daemon: `command`.
    Command(Command),
}
