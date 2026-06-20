//! Self-contained system-prompt message types for OpenRouter agents.
//!
//! Re-defines the system / developer message shapes (content is a plain
//! `String` here — no rich content / parts) plus a [`prepare`] that merges
//! consecutive same-role messages exactly the way
//! [`crate::agent::completions::message::prompt::prepare`] does for the
//! regular prompt: System+System and Developer+Developer collapse (gated by
//! compatible names), their content concatenated with no separator. System
//! and Developer never merge with each other.
//!
//! This is deliberately independent of `agent::completions::message` — it is
//! groundwork for a larger refactor.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A system message setting context or instructions.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.openrouter.SystemMessage")]
pub struct SystemMessage {
    /// The message content.
    pub content: String,
    /// Optional name for the message author.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub name: Option<String>,
}

impl SystemMessage {
    /// Merges `other` into this message (same-role merge): content is
    /// concatenated with no separator, the first name wins.
    pub fn push(&mut self, other: &SystemMessage) {
        self.content.push_str(&other.content);
        if self.name.is_none() {
            self.name.clone_from(&other.name);
        }
    }

    /// Whether this message carries a non-empty name.
    pub fn has_name(&self) -> bool {
        self.name.as_ref().is_some_and(|n| !n.is_empty())
    }

    /// Normalizes the message: an empty-string name becomes `None`.
    pub fn prepare(&mut self) {
        if self.name.as_ref().is_some_and(String::is_empty) {
            self.name = None;
        }
    }
}

/// A developer message (like system, but from the developer).
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.openrouter.DeveloperMessage")]
pub struct DeveloperMessage {
    /// The message content.
    pub content: String,
    /// Optional name for the message author.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub name: Option<String>,
}

impl DeveloperMessage {
    /// Merges `other` into this message (same-role merge): content is
    /// concatenated with no separator, the first name wins.
    pub fn push(&mut self, other: &DeveloperMessage) {
        self.content.push_str(&other.content);
        if self.name.is_none() {
            self.name.clone_from(&other.name);
        }
    }

    /// Whether this message carries a non-empty name.
    pub fn has_name(&self) -> bool {
        self.name.as_ref().is_some_and(|n| !n.is_empty())
    }

    /// Normalizes the message: an empty-string name becomes `None`.
    pub fn prepare(&mut self) {
        if self.name.as_ref().is_some_and(String::is_empty) {
            self.name = None;
        }
    }
}

/// One message of an agent's system prompt — either a system or a developer
/// message. Role-tagged on `role`, mirroring
/// [`crate::agent::completions::message::Message`].
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(tag = "role")]
#[schemars(rename = "agent.openrouter.SystemPromptMessage")]
pub enum SystemPromptMessage {
    /// A developer message.
    #[schemars(title = "Developer")]
    #[serde(rename = "developer")]
    Developer(DeveloperMessage),
    /// A system message.
    #[schemars(title = "System")]
    #[serde(rename = "system")]
    System(SystemMessage),
}

impl SystemPromptMessage {
    /// Normalizes the inner message.
    pub fn prepare(&mut self) {
        match self {
            SystemPromptMessage::Developer(m) => m.prepare(),
            SystemPromptMessage::System(m) => m.prepare(),
        }
    }
}

/// Whether two messages are the same chainable role (system or developer)
/// with compatible names (at most one has a name, or both have the same name).
fn is_chain(a: &SystemPromptMessage, b: &SystemPromptMessage) -> bool {
    match (a, b) {
        (
            SystemPromptMessage::Developer(a),
            SystemPromptMessage::Developer(b),
        ) => !a.has_name() || !b.has_name() || a.name == b.name,
        (SystemPromptMessage::System(a), SystemPromptMessage::System(b)) => {
            !a.has_name() || !b.has_name() || a.name == b.name
        }
        _ => false,
    }
}

/// Pushes `other` into `target` (same-role merge).
fn push(target: &mut SystemPromptMessage, other: &SystemPromptMessage) {
    match (target, other) {
        (
            SystemPromptMessage::Developer(t),
            SystemPromptMessage::Developer(o),
        ) => t.push(o),
        (SystemPromptMessage::System(t), SystemPromptMessage::System(o)) => {
            t.push(o)
        }
        _ => unreachable!(),
    }
}

/// Prepares a system prompt by normalizing each message, then merging chains
/// of consecutive same-role developer/system messages (only when at most one
/// message in the chain has a name). Same algorithm as
/// [`crate::agent::completions::message::prompt::prepare`].
pub fn prepare(messages: &mut Vec<SystemPromptMessage>) {
    messages.iter_mut().for_each(SystemPromptMessage::prepare);

    // scan for any chain to avoid allocation if none exist
    let has_chain = messages.windows(2).any(|w| is_chain(&w[0], &w[1]));
    if !has_chain {
        return;
    }

    let mut merged = Vec::with_capacity(messages.len());
    for msg in messages.drain(..) {
        if let Some(last) = merged.last_mut() {
            if is_chain(last, &msg) {
                push(last, &msg);
                continue;
            }
        }
        merged.push(msg);
    }
    *messages = merged;

    // re-prepare after merging
    prepare(messages);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn system(content: &str, name: Option<&str>) -> SystemPromptMessage {
        SystemPromptMessage::System(SystemMessage {
            content: content.to_string(),
            name: name.map(str::to_string),
        })
    }

    fn developer(content: &str, name: Option<&str>) -> SystemPromptMessage {
        SystemPromptMessage::Developer(DeveloperMessage {
            content: content.to_string(),
            name: name.map(str::to_string),
        })
    }

    #[test]
    fn merges_consecutive_same_role_with_no_separator() {
        let mut msgs = vec![system("a", None), system("b", None)];
        prepare(&mut msgs);
        assert_eq!(msgs, vec![system("ab", None)]);
    }

    #[test]
    fn does_not_merge_across_roles() {
        let mut msgs = vec![system("a", None), developer("b", None)];
        prepare(&mut msgs);
        assert_eq!(msgs, vec![system("a", None), developer("b", None)]);
    }

    #[test]
    fn does_not_merge_incompatible_names() {
        let mut msgs = vec![system("a", Some("x")), system("b", Some("y"))];
        prepare(&mut msgs);
        assert_eq!(msgs, vec![system("a", Some("x")), system("b", Some("y"))]);
    }

    #[test]
    fn merges_compatible_names_first_name_wins() {
        let mut msgs = vec![system("a", Some("x")), system("b", None)];
        prepare(&mut msgs);
        assert_eq!(msgs, vec![system("ab", Some("x"))]);
    }

    #[test]
    fn empty_name_normalizes_to_none() {
        let mut msgs = vec![system("a", Some(""))];
        prepare(&mut msgs);
        assert_eq!(msgs, vec![system("a", None)]);
    }
}
