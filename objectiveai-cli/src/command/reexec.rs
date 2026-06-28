//! Re-exec envelope hygiene.
//!
//! When a CLI `execute` handler exec's a detached child of this CLI
//! (via the SDK's `BinaryExecutor`) by copying its own request onto
//! the child, the child must NOT inherit the parent's output
//! transform or token budget. Only the originating top-level
//! invocation carries those.
//!
//! The rule, applied identically at every such re-exec:
//!
//! | base field      | child inherits? | why                          |
//! |-----------------|-----------------|------------------------------|
//! | `jq`            | NO (stripped)   | output transform, parent-only |
//! | `python`        | NO (stripped)   | output transform, parent-only |
//! | `max_tokens`    | NO (stripped)   | token budget, parent-only     |
//! | `timeout_seconds` | YES (kept)    | the one cap a child honors    |
//!
//! One deliberate non-participant:
//! - `tools run` / `plugins run` launch foreign tool/plugin binaries
//!   (not a re-exec of this CLI) and pass the envelope through
//!   verbatim.

use objectiveai_sdk::cli::command::RequestBase;

/// Strip the parent-only envelope fields (`jq`, `python`,
/// `max_tokens`) from a typed child request's base, leaving
/// `timeout_seconds` intact. Called at every `BinaryExecutor`
/// re-exec site on the child request before it's handed to the
/// executor.
pub fn strip_inherited(base: &mut RequestBase) {
    base.jq = None;
    base.python = None;
    base.max_tokens = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_inherited_keeps_only_timeout() {
        let mut base = RequestBase {
            jq: Some(".x".to_string()),
            python: Some("y".to_string()),
            timeout_seconds: Some(30),
            max_tokens: Some(100),
        };
        strip_inherited(&mut base);
        assert_eq!(base.jq, None);
        assert_eq!(base.python, None);
        assert_eq!(base.max_tokens, None);
        assert_eq!(base.timeout_seconds, Some(30));
    }
}
