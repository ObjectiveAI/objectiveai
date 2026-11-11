//! The kinds of item a tool's log holds, by name.

use serde::{Deserialize, Serialize};

use super::Item;

/// One kind of [`Item`], as its `type` names it: what a request's
/// `type` picks by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ItemType {
    /// [`Item::Deployed`].
    Deployed,
    /// [`Item::Active`].
    Active,
    /// [`Item::Inactive`].
    Inactive,
    /// [`Item::Undeployed`].
    Undeployed,
    /// [`Item::Call`].
    Call,
    /// [`Item::CallResult`].
    CallResult,
    /// [`Item::CallError`].
    CallError,
    /// [`Item::CallInterrupted`].
    CallInterrupted,
    /// [`Item::Read`].
    Read,
    /// [`Item::ReadResult`].
    ReadResult,
    /// [`Item::ReadError`].
    ReadError,
    /// [`Item::ReadInterrupted`].
    ReadInterrupted,
    /// [`Item::Notification`].
    Notification,
    /// [`Item::Error`].
    Error,
    /// [`Item::Command`].
    Command,
}

impl ItemType {
    /// The kind of `item`.
    pub fn of(item: &Item) -> ItemType {
        match item {
            Item::Deployed(_) => ItemType::Deployed,
            Item::Active => ItemType::Active,
            Item::Inactive => ItemType::Inactive,
            Item::Undeployed(_) => ItemType::Undeployed,
            Item::Call(_) => ItemType::Call,
            Item::CallResult(_) => ItemType::CallResult,
            Item::CallError(_) => ItemType::CallError,
            Item::CallInterrupted(_) => ItemType::CallInterrupted,
            Item::Read(_) => ItemType::Read,
            Item::ReadResult(_) => ItemType::ReadResult,
            Item::ReadError(_) => ItemType::ReadError,
            Item::ReadInterrupted(_) => ItemType::ReadInterrupted,
            Item::Notification(_) => ItemType::Notification,
            Item::Error(_) => ItemType::Error,
            Item::Command(_) => ItemType::Command,
        }
    }

    /// The kind's name, as `type` carries it on the wire.
    pub fn name(self) -> &'static str {
        match self {
            ItemType::Deployed => "deployed",
            ItemType::Active => "active",
            ItemType::Inactive => "inactive",
            ItemType::Undeployed => "undeployed",
            ItemType::Call => "call",
            ItemType::CallResult => "call_result",
            ItemType::CallError => "call_error",
            ItemType::CallInterrupted => "call_interrupted",
            ItemType::Read => "read",
            ItemType::ReadResult => "read_result",
            ItemType::ReadError => "read_error",
            ItemType::ReadInterrupted => "read_interrupted",
            ItemType::Notification => "notification",
            ItemType::Error => "error",
            ItemType::Command => "command",
        }
    }
}
