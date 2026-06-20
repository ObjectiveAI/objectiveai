//! Shared format + parse for the `<system-reminder-{token}>` wrapper
//! that the MCP proxy prepends to tool responses when surfacing
//! pending `message_queue` content. The proxy emits via
//! [`format_prefix`]; `run_agent_loop` (in `objectiveai-api`)
//! matches via [`extract_tokens`]. Owning both in one module
//! keeps the two ends in lockstep — a format change here
//! updates the matcher implicitly.
//!
//! The confirmation token is embedded directly in the opening tag
//! name: `<system-reminder-<UUID>>`. The API delegate generates the
//! token on every `read_pending_blocks` call and stashes the
//! `token → ids` mapping until the run-loop sees the token in a tool
//! message and confirms delivery. Tokens never echoed back stay in
//! "pending" limbo and re-deliver on the next loop's reads —
//! that's the robustness win over a naive ban-list-only design.

/// Format the wrapper opening tag the proxy prepends to a tool
/// response when surfacing queued blocks. The token is embedded in
/// the tag name and is opaque to the proxy — it round-trips through
/// the agent's tool-message text to `run_agent_loop`'s confirmation
/// scan.
pub fn format_prefix(token: &str) -> String {
    format!("<system-reminder-{token}>\nThe user sent a new message while you were working:\n")
}

/// Format the matching closing tag. The token is embedded in the tag
/// name too, so the closing tag pairs with [`format_prefix`]'s opening
/// tag: `</system-reminder-<UUID>>`.
pub fn format_suffix(token: &str) -> String {
    format!("\n\n</system-reminder-{token}>")
}

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
        // Match just the opening tag and capture the UUID token in its
        // name. The human-readable line that follows the tag is
        // ignored — only `<system-reminder-{token}>` is matched. Not
        // anchored to start/end and no newline requirement (the model
        // may reflow whitespace around the tag), but the token keeps
        // the strict UUID v4 shape (lowercase hex, 8-4-4-4-12) so
        // arbitrary tool output can't be mistaken for a token. The
        // pattern needs `<` immediately followed by `system-reminder-`,
        // so the `</system-reminder-{token}>` closing tag (a `/` follows
        // its `<`) is never captured — even though both tags now carry
        // the token, only the opening one is extracted.
        regex::Regex::new(
            r"<system-reminder-([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})>",
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
        let t1 = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
        let t2 = "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb";
        let body = format!(
            "{p1}block one{s1}{p2}block two{s2}",
            p1 = format_prefix(t1),
            p2 = format_prefix(t2),
            s1 = format_suffix(t1),
            s2 = format_suffix(t2),
        );
        let tokens = extract_tokens(&body);
        assert_eq!(tokens, vec![t1.to_string(), t2.to_string()]);
    }

    #[test]
    fn closing_tag_token_is_not_extracted() {
        // A full open+close round-trip yields exactly the opening tag's
        // token — the token-bearing closing tag must not be captured too.
        let token = "12345678-1234-1234-1234-1234567890ab";
        let body =
            format!("{}body{}", format_prefix(token), format_suffix(token));
        assert_eq!(extract_tokens(&body), vec![token.to_string()]);
    }

    #[test]
    fn uppercase_hex_does_not_match() {
        // UUID v4 in the regex is lowercase-only. Catches accidental
        // case drift in the format function.
        let prefix = "<system-reminder-ABCDEF01-1234-1234-1234-1234567890AB>";
        assert!(extract_tokens(prefix).is_empty());
    }
}
