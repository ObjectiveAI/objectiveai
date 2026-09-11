//! The Python agent.

use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::Version;

/// An agent that runs Python instead of calling a model.
///
/// No `model` field, which is the point: nothing is sampled, so there
/// is nothing to name — and no sampling parameters either. A Python
/// agent occupies the same slot as a model-backed one and answers
/// deterministically, or as deterministically as its source does.
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default,
)]
pub struct Agent {
    /// The source, verbatim.
    ///
    /// Never normalized — whitespace is significant in Python, so the
    /// trimming applied to most string fields would change what the
    /// code means. Written to disk as it is at registration, and run
    /// as it is every turn.
    pub python: String,
    /// Third-party packages the source needs: distribution name to
    /// version constraint. Installed at registration, once.
    ///
    /// Precision is the author's statement of intent: a loose
    /// specifier says "track updates", an exact one says "freeze
    /// this". A map rather than a list of requirement lines, so one
    /// package cannot appear twice with constraints that contradict
    /// each other. Ordered, so the same set always serializes — and
    /// installs — identically instead of shuffling between runs.
    ///
    /// One constraint per package, which is what the shape costs. A
    /// compound range (`>= 2, < 3`) and an environment marker
    /// (`; python_version < "3.11"`) are both unrepresentable — the
    /// first needs two constraints for one name, the second needs a
    /// place to put the marker. Both are legal in a
    /// `requirements.txt` and neither survives here.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub requirements: IndexMap<String, Version>,
}

impl Agent {
    /// The requirements as pip takes them: one `name<op>version`
    /// argument each, in the map's order.
    pub fn pip_arguments(&self) -> Vec<String> {
        self.requirements
            .iter()
            .map(|(name, version)| format!("{name}{version}"))
            .collect()
    }
}
