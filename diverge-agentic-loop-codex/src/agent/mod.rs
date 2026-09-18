//! The agent: Codex's parameters, as the caller states them.
//!
//! This is what the `arguments` on the container's request must
//! be, and every type here derives its JSON Schema so `GET /schema`
//! can say so — [`schemars::schema_for!`] over [`Agent`] is the
//! whole answer.
//!
//! Codex (OpenAI's Codex CLI) is an agent harness in its own right,
//! driven here as `codex exec --json` per turn with a `config.toml`
//! the harness writes under `CODEX_HOME`. Every field here is
//! HONORABLE: it names a key of that file, or a flag of that command,
//! that the harness actually renders — `model`,
//! `model_reasoning_effort`, `model_reasoning_summary`,
//! `model_verbosity`, `web_search`, and a `model_providers` entry —
//! and nothing else made the cut. Nothing here is a secret, and
//! nothing here says how Codex logs in: the run looks for an
//! `auth.json` the caller MOUNTED at `$CODEX_HOME` first — a FUSE
//! mount on the container request, served live by the caller — and,
//! absent one, for the vault's static `OPENAI_API_KEY`, put into the
//! process environment; it refuses the run only when both are
//! missing.
//!
//! What is deliberately ABSENT, and why, so nobody re-opens it
//! without a new fact:
//! - SANDBOX AND APPROVAL (`sandbox_mode`, `approval_policy`,
//!   `--sandbox`, `--ask-for-approval`): the container is the
//!   sandbox. The harness always runs `danger-full-access` with
//!   `approval_policy = "never"` — cc's skipped permissions and
//!   Hermes's yolo mode, by another name — because a prompt waits on
//!   a user who is not there, and a sandbox inside a sandbox protects
//!   nothing.
//! - MCP SERVERS: always exactly one, `mcp_servers.diverge` at the
//!   container SDK's `mcp_url()` — the caller's tools and resources,
//!   through the proxy, which is what the protocol is built around.
//! - SESSION PERSISTENCE (`--ephemeral`, `history.persistence`): the
//!   continuation is the harness's; `exec resume` is how a run picks
//!   the conversation up.
//! - `hide_agent_reasoning` and `show_raw_agent_reasoning`: the
//!   stream wants the reasoning, and gets what
//!   [`reasoning_summary`](Agent::reasoning_summary) allows.
//! - IMAGES and OUTPUT SCHEMAS (`--image`, `--output-schema`): the
//!   wire is text in and chunks out.
//! - PROFILES, `notify` hooks, execpolicy rules and
//!   `shell_environment_policy`: the harness's rendering of the
//!   process, not the caller's vocabulary.
//! - THE LOGIN (`forced_login_method`, any credential): a mount or
//!   the vault's key, above — the caller who mounted an `auth.json`
//!   chose its kind, and the harness never reads it.
//! - SKILLS: mounts, exactly as for Claude Code; a directory under
//!   Codex's skills path is a skill, and no switch decides it.
//! - `wire_api`: the configuration reference names one value,
//!   `responses`, and the harness writes it.

mod agent;
mod effort;
mod provider;
mod summary;
mod verbosity;
mod web_search;

pub use agent::*;
pub use effort::*;
pub use provider::*;
pub use summary::*;
pub use verbosity::*;
pub use web_search::*;
