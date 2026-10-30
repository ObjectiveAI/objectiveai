//! The kinds of item a tool's log holds, by name.

use serde::{Deserialize, Serialize};

use super::Item;

/// One kind of [`Item`], as its `type` names it: what a request's
/// `type` picks by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ItemType {
    /// [`Item::Active`].
    Active,
    /// [`Item::Inactive`].
    Inactive,
    /// [`Item::Call`].
    Call,
    /// [`Item::CallResult`].
    CallResult,
    /// [`Item::CallError`].
    CallError,
    /// [`Item::Read`].
    Read,
    /// [`Item::ReadResult`].
    ReadResult,
    /// [`Item::ReadError`].
    ReadError,
    /// [`Item::Notification`].
    Notification,
    /// [`Item::Error`].
    Error,
}

impl ItemType {
    /// The kind of `item`.
    pub fn of(item: &Item) -> ItemType {
        match item {
            Item::Active => ItemType::Active,
            Item::Inactive => ItemType::Inactive,
            Item::Call(_) => ItemType::Call,
            Item::CallResult(_) => ItemType::CallResult,
            Item::CallError(_) => ItemType::CallError,
            Item::Read(_) => ItemType::Read,
            Item::ReadResult(_) => ItemType::ReadResult,
            Item::ReadError(_) => ItemType::ReadError,
            Item::Notification(_) => ItemType::Notification,
            Item::Error(_) => ItemType::Error,
        }
    }
}
