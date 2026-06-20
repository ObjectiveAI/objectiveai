//! The agent's system prompt: an ordered list of role-tagged text entries.
//!
//! Each [`SystemPrompt`] is a `role` (`system` or `developer`) plus plain
//! `String` `content`. [`prepare`] collapses consecutive same-role entries by
//! concatenating their content (no separator), keeping the list canonical for
//! the agent's content-addressed id.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The role of a [`SystemPrompt`] entry.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "agent.openrouter.SystemPromptRole")]
pub enum SystemPromptRole {
    System,
    Developer,
}

/// One entry of an agent's system prompt — a role and its text content.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.openrouter.SystemPrompt")]
pub struct SystemPrompt {
    /// Whether this entry is a system or developer message.
    pub role: SystemPromptRole,
    /// The entry's text content.
    pub content: String,
}

/// Merges consecutive same-role entries, concatenating their content with no
/// separator. One left-fold collapses every run — no recursion is needed since
/// `content` is a plain `String` with nothing further to normalize.
pub fn prepare(prompts: &mut Vec<SystemPrompt>) {
    let has_chain = prompts.windows(2).any(|w| w[0].role == w[1].role);
    if !has_chain {
        return;
    }
    let mut merged: Vec<SystemPrompt> = Vec::with_capacity(prompts.len());
    for prompt in prompts.drain(..) {
        if let Some(last) = merged.last_mut() {
            if last.role == prompt.role {
                last.content.push_str(&prompt.content);
                continue;
            }
        }
        merged.push(prompt);
    }
    *prompts = merged;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sp(role: SystemPromptRole, content: &str) -> SystemPrompt {
        SystemPrompt {
            role,
            content: content.to_string(),
        }
    }

    #[test]
    fn merges_consecutive_same_role_with_no_separator() {
        let mut v = vec![
            sp(SystemPromptRole::System, "a"),
            sp(SystemPromptRole::System, "b"),
        ];
        prepare(&mut v);
        assert_eq!(v, vec![sp(SystemPromptRole::System, "ab")]);
    }

    #[test]
    fn does_not_merge_across_roles() {
        let mut v = vec![
            sp(SystemPromptRole::System, "a"),
            sp(SystemPromptRole::Developer, "b"),
        ];
        prepare(&mut v);
        assert_eq!(
            v,
            vec![
                sp(SystemPromptRole::System, "a"),
                sp(SystemPromptRole::Developer, "b"),
            ]
        );
    }

    #[test]
    fn merges_runs_and_preserves_role_boundaries() {
        let mut v = vec![
            sp(SystemPromptRole::System, "a"),
            sp(SystemPromptRole::System, "b"),
            sp(SystemPromptRole::Developer, "c"),
            sp(SystemPromptRole::System, "d"),
        ];
        prepare(&mut v);
        assert_eq!(
            v,
            vec![
                sp(SystemPromptRole::System, "ab"),
                sp(SystemPromptRole::Developer, "c"),
                sp(SystemPromptRole::System, "d"),
            ]
        );
    }
}
