//! A version constraint on a requirement.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::Operator;

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
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, Default,
)]
pub struct Version {
    /// How to compare.
    pub operator: Operator,
    /// What to compare against — a PEP 440 version, without the
    /// operator. `"2.31.0"`, not `">=2.31.0"`.
    ///
    /// Left a string rather than parsed into components: PEP 440
    /// versions carry epochs, pre/post/dev segments and local
    /// identifiers (`1!2.0.0rc1.post2+ubuntu.1`), and a structure that
    /// modelled all of it would be a version parser in a wire type.
    pub version: String,
}

impl std::fmt::Display for Version {
    /// The constraint as a requirement specifier's tail: the operator
    /// then the version, `>=2.31.0`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.operator, self.version)
    }
}
