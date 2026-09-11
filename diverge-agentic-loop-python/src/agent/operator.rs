//! How a version constraint compares.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// How a [`Version`](super::Version) compares, as PEP 440 defines
/// them.
///
/// A closed set, so it is an enum rather than a string: an operator
/// pip does not understand fails to deserialize instead of failing
/// at install time.
///
/// Seven of PEP 440's eight. `===`, arbitrary equality, is omitted —
/// it exists for versions that are not PEP 440 versions, and PyPI
/// enforces PEP 440 on upload, so nothing reachable from the default
/// index can need it.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    JsonSchema,
)]
pub enum Operator {
    /// `==` — this version. The default, because freezing is the
    /// safer thing to do by accident.
    #[serde(rename = "==")]
    #[default]
    Equal,
    /// `!=` — anything but this version.
    #[serde(rename = "!=")]
    NotEqual,
    /// `>=` — this version or newer. The usual way to say "track
    /// updates".
    #[serde(rename = ">=")]
    GreaterEqual,
    /// `>` — strictly newer.
    #[serde(rename = ">")]
    Greater,
    /// `<=` — this version or older.
    #[serde(rename = "<=")]
    LessEqual,
    /// `<` — strictly older.
    #[serde(rename = "<")]
    Less,
    /// `~=` — compatible release. Newer, but not past the last
    /// component: `~= 2.31.0` allows `2.31.4` and refuses `2.32.0`.
    #[serde(rename = "~=")]
    Compatible,
}

impl Operator {
    /// The operator as pip spells it.
    pub fn as_str(self) -> &'static str {
        match self {
            Operator::Equal => "==",
            Operator::NotEqual => "!=",
            Operator::GreaterEqual => ">=",
            Operator::Greater => ">",
            Operator::LessEqual => "<=",
            Operator::Less => "<",
            Operator::Compatible => "~=",
        }
    }
}

impl std::fmt::Display for Operator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
