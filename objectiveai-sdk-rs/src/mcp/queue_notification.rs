//! Shared format + parse for the `<system-reminder>` wrapper that
//! the MCP proxy prepends to tool responses when surfacing
//! pending `message_queue` content. The proxy emits via
//! [`format_prefix`]; `run_agent_loop` (in `objectiveai-api`)
//! matches via [`extract_tokens`]. Owning both in one module
//! keeps the two ends in lockstep — a format change here
//! updates the matcher implicitly.
//!
//! The prefix embeds a confirmation token inline as `(id: <UUID>)`.
//! The API delegate generates the token on every
//! `read_pending_blocks` call and stashes the `token → ids`
//! mapping until the run-loop sees the token in a tool message
//! and confirms delivery. Tokens never echoed back stay in
//! "pending" limbo and re-deliver on the next loop's reads —
//! that's the robustness win over a naive ban-list-only design.

/// Format the wrapper prefix the proxy prepends to a tool
/// response when surfacing queued blocks. The token is opaque to
/// the proxy — it round-trips through the agent's tool-message
/// text to `run_agent_loop`'s confirmation scan.
pub fn format_prefix(token: &str) -> String {
    format!(
        "<system-reminder>\n\
         The user sent a new message while you were working: (id: {token})\n",
    )
}

/// The matching suffix. Token-independent; mirrors the historic
/// proxy wrapper shape so agents trained on the old format
/// recognize the closing tag.
pub const SUFFIX: &str = "\n\n</system-reminder>";

/// Scan one text chunk for the prefix pattern; return every
/// captured token in document order. Typical case is zero or one
/// match per tool message; multiple are possible (one delegate
/// call per tool call, but the proxy could splice multiple
/// reminders in pathological scenarios), and the regex captures
/// all to be safe.
///
/// The regex is compiled lazily — first call costs a few µs to
/// build the DFA; subsequent calls reuse the cached `Regex` via
/// `OnceLock` with zero overhead.
pub fn extract_tokens(text: &str) -> Vec<String> {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let re = RE.get_or_init(|| {
        // Anchored to `format_prefix`'s exact output. Token is
        // UUID v4 (lowercase hex + hyphens, 8-4-4-4-12). Strict
        // pattern avoids false positives against arbitrary
        // tool-output text that happens to mention parens.
        regex::Regex::new(
            r"<system-reminder>\nThe user sent a new message while you were working: \(id: ([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})\)\n",
        )
        .expect("static regex pattern is well-formed")
    });
    re.captures_iter(text)
        .map(|c| c.get(1).expect("group 1 is present in pattern").as_str().to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_and_extract_round_trip() {
        let token = "12345678-1234-1234-1234-1234567890ab";
        let prefix = format_prefix(token);
        let extracted = extract_tokens(&prefix);
        assert_eq!(extracted, vec![token.to_string()]);
    }

    #[test]
    fn no_match_on_unrelated_text() {
        assert!(extract_tokens("plain old tool output").is_empty());
        assert!(extract_tokens("(id: bogus)").is_empty());
        assert!(extract_tokens("<system-reminder>\nDifferent text\n").is_empty());
    }

    #[test]
    fn extracts_multiple_tokens() {
        let body = format!(
            "{p1}block one{s}{p2}block two{s}",
            p1 = format_prefix("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa"),
            p2 = format_prefix("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb"),
            s = SUFFIX,
        );
        let tokens = extract_tokens(&body);
        assert_eq!(
            tokens,
            vec![
                "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa".to_string(),
                "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb".to_string(),
            ]
        );
    }

    #[test]
    fn uppercase_hex_does_not_match() {
        // UUID v4 in the regex is lowercase-only. Catches accidental
        // case drift in the format function.
        let prefix = "<system-reminder>\nThe user sent a new message while you were working: (id: ABCDEF01-1234-1234-1234-1234567890AB)\n";
        assert!(extract_tokens(prefix).is_empty());
    }
}
