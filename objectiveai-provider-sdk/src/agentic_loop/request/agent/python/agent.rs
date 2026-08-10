//! The Python agent.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::{Upstream, Version};

/// An agent that runs Python instead of calling a model.
///
/// No `model` field, which is the point: nothing is sampled, so there
/// is nothing to name — and no sampling parameters either. A Python
/// agent occupies the same slot as a model-backed one and answers
/// deterministically.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Agent {
    /// The discriminator. Always `python`.
    pub upstream: Upstream,
    /// The source, verbatim.
    ///
    /// Never normalized — whitespace is significant in Python, so the
    /// trimming applied to most string fields would change what the
    /// code means.
    pub python: String,
    /// Third-party packages the source needs: distribution name to
    /// version constraint.
    ///
    /// Precision is the author's statement of intent, exactly as it is
    /// for [`model`](super::super::openrouter::Agent::model) one
    /// upstream over: a loose specifier says "track updates", an exact
    /// one says "freeze this". Content addressing identifies the
    /// definition, not the execution — an id means two runs were given
    /// the same instructions, never that the world outside was the
    /// same.
    ///
    /// A map rather than a list of requirement lines, so one package
    /// cannot appear twice with constraints that contradict each
    /// other. Ordered, so the same set always serializes identically
    /// instead of shuffling between runs.
    ///
    /// One constraint per package, which is what the shape costs.
    /// A compound range (`>= 2, < 3`) and an environment marker
    /// (`; python_version < "3.11"`) are both unrepresentable — the
    /// first needs two constraints for one name, the second needs a
    /// place to put the marker. Both are legal in a `requirements.txt`
    /// and neither survives here.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub requirements: IndexMap<String, Version>,
}
