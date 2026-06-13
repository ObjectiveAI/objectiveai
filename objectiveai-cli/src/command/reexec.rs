//! Re-exec / nested-run envelope hygiene.
//!
//! When a CLI `execute` handler hands work to a child of the CLI —
//! either by exec'ing a detached CLI subprocess (via the SDK's
//! `BinaryExecutor`) or by re-entering the top-level entry in-process
//! (`crate::run`) — the child must NOT inherit the parent's output
//! transform or token budget. Only the originating top-level
//! invocation carries those.
//!
//! The rule, applied identically at every such handoff:
//!
//! | base field      | child inherits? | why                          |
//! |-----------------|-----------------|------------------------------|
//! | `jq`            | NO (stripped)   | output transform, parent-only |
//! | `python`        | NO (stripped)   | output transform, parent-only |
//! | `max_tokens`    | NO (stripped)   | token budget, parent-only     |
//! | `timeout_seconds` | YES (kept)    | the one cap a child honors    |
//!
//! `tools run` / `plugins run` are the deliberate exception: they
//! launch foreign tool/plugin binaries (not a re-exec of this CLI),
//! pass the envelope through verbatim, and never route through here.

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

/// Argv form of [`strip_inherited`] for the `crate::run` handoff,
/// where the child command is already serialized to arguments.
/// Removes `--jq`, `--python`, `--max-tokens` and each one's value;
/// `--timeout` survives. Handles both the `--flag value` and
/// `--flag=value` spellings clap accepts.
pub fn strip_inherited_args(args: &mut Vec<String>) {
    const STRIP: [&str; 3] = ["--jq", "--python", "--max-tokens"];
    let mut out = Vec::with_capacity(args.len());
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        // `--flag value`: drop the flag token AND its separate value.
        if STRIP.contains(&arg) {
            i += 2;
            continue;
        }
        // `--flag=value`: drop just this one token.
        if STRIP.iter().any(|f| arg.starts_with(f) && arg[f.len()..].starts_with('=')) {
            i += 1;
            continue;
        }
        out.push(args[i].clone());
        i += 1;
    }
    *args = out;
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

    #[test]
    fn strip_args_space_form() {
        let mut args = vec![
            "agents".to_string(),
            "spawn".to_string(),
            "--jq".to_string(),
            ".foo".to_string(),
            "--timeout".to_string(),
            "30s".to_string(),
            "--python".to_string(),
            "code".to_string(),
            "--max-tokens".to_string(),
            "100".to_string(),
        ];
        strip_inherited_args(&mut args);
        assert_eq!(
            args,
            vec![
                "agents".to_string(),
                "spawn".to_string(),
                "--timeout".to_string(),
                "30s".to_string(),
            ]
        );
    }

    #[test]
    fn strip_args_equals_form() {
        let mut args = vec![
            "--jq=.foo".to_string(),
            "--timeout=30s".to_string(),
            "--max-tokens=100".to_string(),
        ];
        strip_inherited_args(&mut args);
        assert_eq!(args, vec!["--timeout=30s".to_string()]);
    }
}
