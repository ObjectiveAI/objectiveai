//! What one entry of a tool's log holds.

use serde::{Deserialize, Serialize};

use super::{Call, CallError, CallResult, Error, Notification, Read, ReadError, ReadResult};
use crate::daemon::command_log::Command;

/// One item of a tool's log, told apart by `type`.
///
/// The rules:
///
/// 1. An ask — a [`Call`] or a [`Read`] — is kept when the daemon
///    routes it, before the tool is asked; its answer — a result or an
///    error — when the tool answers, naming the ask by its
///    `logs_index`. Every ask is answered exactly once in the log,
///    unless the daemon stops first.
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
    /// An ask came in while none was in flight: `active`.
    Active,
    /// The last ask in flight was answered: `inactive`.
    Inactive,
    /// A tool call: `call`.
    Call(Call),
    /// A tool call's result: `call_result`.
    CallResult(CallResult),
    /// A tool call that failed: `call_error`.
    CallError(CallError),
    /// A resource read: `read`.
    Read(Read),
    /// A resource read's result: `read_result`.
    ReadResult(ReadResult),
    /// A resource read that failed: `read_error`.
    ReadError(ReadError),
    /// A notification the tool sent: `notification`.
    Notification(Notification),
    /// An error the daemon met keeping the tool: `error`.
    Error(Error),
    /// A request the tool put to the daemon: `command`.
    Command(Command),
}
