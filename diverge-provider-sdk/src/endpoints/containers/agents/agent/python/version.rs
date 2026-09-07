//! A version constraint on a requirement.

use serde::{Deserialize, Serialize};

/// A constraint on which version of a package satisfies a
/// requirement.
///
/// Split into an operator and a version rather than kept as one
/// string, so the operator is a closed set a consumer matches on
/// instead of a prefix it has to parse off the front. `">==2.0"` is
/// not representable.
///
/// There is no "any version" — an operator is always required. Say
/// `>= 0` and mean it, rather than leaving a consumer to decide what
/// an absent constraint implies.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Version {
    /// How to compare.
    pub operator: VersionOperator,
    /// What to compare against — a PEP 440 version, without the
    /// operator. `"2.31.0"`, not `">=2.31.0"`.
    ///
    /// Left a string rather than parsed into components: PEP 440
    /// versions carry epochs, pre/post/dev segments and local
    /// identifiers (`1!2.0.0rc1.post2+ubuntu.1`), and a structure that
    /// modelled all of it would be a version parser in a wire type.
    pub version: String,
}

/// How a [`Version`] compares, as PEP 440 defines them.
///
/// A closed set, so it is an enum rather than a string: an operator no
/// resolver understands fails to deserialize instead of failing later.
///
/// Seven of PEP 440's eight. `===`, arbitrary equality, is omitted —
/// it exists for versions that are not PEP 440 versions, and PyPI
/// enforces PEP 440 on upload, so nothing reachable from the default
/// index can need it. It belongs with an index field, if one ever
/// lands.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub enum VersionOperator {
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
