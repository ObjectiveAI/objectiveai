//! Which kind of content is being asked for.

use serde::{Deserialize, Serialize};

/// What the dirhash identifies.
///
/// The hash alone says WHICH content; this says what the content is,
/// which is what tells the client which of its folders to look in —
/// skills live in one, agent definitions in another, and the two
/// cannot share because their formats collide.
///
/// # Named for the format, like the fields that carry the hashes
///
/// [`ClaudeCodeAgent`](Self::ClaudeCodeAgent) rather than a bare
/// `Agent`, because agent definitions are format-specific where
/// skills are not: a skill folder serves any harness, but Claude
/// Code's markdown-with-frontmatter definitions and Codex's TOML are
/// different kinds, and a future `codex_agent` is a new variant here
/// rather than a new meaning for an old one.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// A skill: a directory whose `SKILL.md` leads it.
    Skill,
    /// A Claude Code agent definition.
    ClaudeCodeAgent,
}
