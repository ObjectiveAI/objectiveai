//! One operation of a patch, and the patch as a list of them.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A JSON Patch: operations applied in order to one document, whole
/// or not at all.
pub type Patch = Vec<Operation>;

/// One operation of RFC 6902, on one place in the document. JSON
/// tagged by `op`: `{"op":"add","path":"/x/-","value":…}`.
///
/// # The path
///
/// `path`, and `from`, are JSON Pointers, RFC 6901: the empty string
/// is the whole document; `/a/b` the member `b` of the member `a`;
/// `/list/0` the first element of `list`, `/list/-` the place after
/// its last, which only an `add` may name; `~1` in a token is `/` and
/// `~0` is `~`. A pointer that is not one — no leading `/`, a bad
/// escape — fails the patch.
///
/// # What each does
///
/// `add` puts the value at the path: into an object, as the member,
/// made or replaced; into a list, inserted at the index, the rest
/// moving up, or at `-` appended; at the empty path, the whole
/// document replaced. `remove` takes away what is at the path, which
/// must exist. `replace` puts the value where one already is. `move`
/// takes what is at `from` and adds it at `path`, which may not be
/// inside `from`. `copy` adds at `path` what is at `from`. `test`
/// fails the patch unless what is at the path equals the value, as
/// JSON values are equal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Operation {
    /// Put `value` at `path`: made or replaced in an object, inserted
    /// or appended in a list.
    Add {
        /// Where.
        path: String,
        /// What.
        value: Value,
    },
    /// Take away what is at `path`, which must exist.
    Remove {
        /// Where.
        path: String,
    },
    /// Put `value` where something already is at `path`.
    Replace {
        /// Where.
        path: String,
        /// What.
        value: Value,
    },
    /// Take away what is at `from` and add it at `path`.
    Move {
        /// Where from.
        from: String,
        /// Where to, not inside `from`.
        path: String,
    },
    /// Add at `path` a copy of what is at `from`.
    Copy {
        /// Where from.
        from: String,
        /// Where to.
        path: String,
    },
    /// Fail the patch unless what is at `path` equals `value`.
    Test {
        /// Where.
        path: String,
        /// What it must be.
        value: Value,
    },
}

impl Operation {
    /// The operation's name, as `op` spells it.
    pub fn name(&self) -> &'static str {
        match self {
            Operation::Add { .. } => "add",
            Operation::Remove { .. } => "remove",
            Operation::Replace { .. } => "replace",
            Operation::Move { .. } => "move",
            Operation::Copy { .. } => "copy",
            Operation::Test { .. } => "test",
        }
    }
}
