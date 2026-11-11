//! A member an edit may leave, clear or replace.
//!
//! An edit names each member three ways: absent leaves the record's,
//! `null` clears it, and a value replaces it. As
//! `Option<Option<T>>`: `None` absent, `Some(None)` cleared,
//! `Some(Some(value))` replaced. Serde reads a present `null` as the
//! outer `None` unless told otherwise; [`deserialize`] tells it, with
//! `#[serde(default, deserialize_with = "nullable::deserialize")]` on
//! the member, so absence is the default and `null` is `Some(None)`.

use serde::{Deserialize, Deserializer};

/// A present member: `null` as `Some(None)`, a value as
/// `Some(Some(value))`. Absence never reaches here: the member's
/// `default` makes it `None`.
pub fn deserialize<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}
