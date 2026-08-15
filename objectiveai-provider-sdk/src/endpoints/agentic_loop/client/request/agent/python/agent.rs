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
    /// How much memory the source needs, in BYTES.
    ///
    /// Not a hint. A Python agent runs in a container, and this is the
    /// ceiling that container is given — so a process that exceeds
    /// what it is allowed is killed by the kernel rather than told to
    /// try something else. There is no failed allocation to catch and
    /// no warning first.
    ///
    /// Which makes this the author's job and nobody else's. A provider
    /// cannot infer it: the source is opaque until it runs, and by
    /// then the number is already needed. Guessing high wastes a
    /// provider's capacity on every run; guessing low kills the agent
    /// partway through its work.
    ///
    /// Bytes rather than megabytes because a unit that has to be
    /// spelled out in prose is a unit half of everyone gets wrong.
    ///
    /// Note that the interpreter is included. This is what the
    /// CONTAINER may use, not what the script allocates on top of a
    /// runtime somebody else is paying for.
    pub memory: u64,
    /// How much the source may WRITE, in BYTES.
    ///
    /// The container's own filesystem — what it adds to or changes
    /// over the image it came from. The image's layers are read-only
    /// and are not counted, so a run starts at nothing however large
    /// the interpreter and its packages are.
    ///
    /// The author's job for the same reason
    /// [`memory`](Self::memory) is: a provider cannot infer what a
    /// script will write, because the source is opaque until it runs
    /// and by then the number is already needed.
    ///
    /// What [`requirements`](Self::requirements) install is part of
    /// the image rather than part of this. They are resolved before
    /// the source runs, so a run does not spend its allowance on its
    /// own dependencies.
    pub disk: u64,
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
