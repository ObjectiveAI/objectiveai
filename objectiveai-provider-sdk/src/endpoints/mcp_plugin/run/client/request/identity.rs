//! On whose behalf a plugin is being run.

use serde::{Deserialize, Serialize};

/// Who is calling, and about what.
///
/// A plugin serves tools to an agent, and some of what it serves
/// depends on which agent — a plugin that keeps state per caller, or
/// bills per caller, or refuses one, needs to be told. This is that
/// telling, and it is fixed for the container's whole life: a plugin
/// container is created for one caller and torn down after, so nothing
/// here changes between one tool call and the next.
///
/// Which is why it belongs in the run request rather than on each
/// exchange. Putting it on the exchange would let it vary, and a
/// plugin that reads it once at startup — as the reference
/// implementation does — would silently ignore the variation.
///
/// # Almost all of it is optional
///
/// Because almost all of it may be genuinely unknown. A plugin invoked
/// outside an agentic loop has no loop to name; one invoked directly
/// belongs to no agent. Absence is a real answer here, not a caller
/// being lazy, and a plugin that requires a field says so in its own
/// terms rather than being handed a placeholder.
///
/// [`agent_instance`](Self::agent_instance) is the exception. Every
/// call comes from somewhere, and that somewhere is what a plugin
/// keying anything per-caller keys on.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Identity {
    /// Which instance is calling.
    ///
    /// The leaf of the calling chain, and the only part of it that is
    /// always known. A plugin holding state per caller holds it under
    /// this.
    pub agent_instance: String,
    /// What called that, outermost first.
    ///
    /// The chain above [`agent_instance`](Self::agent_instance), which
    /// exists because agents call agents: a plugin reached through
    /// three layers of delegation can see all three, and decide
    /// whether it cares.
    ///
    /// `None` and an empty vector are different. `None` is a caller
    /// that did not say, and an empty vector is a caller that did say
    /// and has no parent — a top-level instance. A plugin that treats
    /// them alike loses the ability to tell "nobody told me" from
    /// "nothing is above this".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_parent_instance: Option<Vec<String>>,
    /// Which agent, by its short identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    /// Which agent, qualified by whoever owns it.
    ///
    /// The unambiguous form. Two owners may both publish an agent with
    /// the same [`agent_id`](Self::agent_id), and this is what tells
    /// them apart.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_full_id: Option<String>,
    /// Where that agent runs, if it is not here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_remote: Option<String>,
    /// Which [`agentic_loop`](crate::endpoints::agentic_loop) this
    /// call is part of.
    ///
    /// The work the agent is doing, as against
    /// [`agent_instance`](Self::agent_instance), which is the agent
    /// doing it. One loop reaches many instances and one instance may
    /// be reached by several loops, so neither implies the other.
    ///
    /// A plugin caching or accumulating across a piece of work keys on
    /// this — it is the widest scope a plugin is told about, and the
    /// one that corresponds to something a person asked for.
    ///
    /// Absent means the call is not part of a loop: a plugin invoked
    /// directly, or by something that is not an agent at all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agentic_loop_id: Option<String>,
    /// Who publishes this plugin.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_owner: Option<String>,
    /// What the plugin is called.
    ///
    /// The plugin's OWN name, not the caller's — this half of the
    /// struct tells a plugin what it is, which it cannot otherwise
    /// know: an image does not carry the name it was published under,
    /// and the same image may be published twice.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_name: Option<String>,
    /// Which version of it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_version: Option<String>,
    /// The task this call serves, if it serves one.
    ///
    /// Present means the call is part of a task and names which;
    /// absent means it is not part of one. A plugin that behaves
    /// differently inside a task — writing results somewhere, or
    /// declining work that has no task to bill — reads this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task: Option<String>,
}
