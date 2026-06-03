//! Data-URL parsing utility shared across the SDK.

/// Parses a data URL, returning `(full_mime, base64_payload)`.
///
/// Expects the format `data:{type}/{subtype};base64,{payload}` and
/// validates that **the entire input is a single data URL** — the
/// payload must be exclusively standard base64 characters
/// (`[A-Za-z0-9+/=]`) and the mime carries no whitespace. That
/// rules out strings where a data-URL prefix is followed by
/// unrelated text (e.g. a tool output that happens to start with
/// `data:image/png;base64,XYZ\nfollowed by prose…`); those round-
/// trip as `None` so callers reliably pass them through as text.
///
/// Returns `None` for:
/// - Strings missing the `data:` prefix (including any leading
///   whitespace or content before it).
/// - Strings missing the `;base64,` marker.
/// - Strings whose payload contains anything outside the standard
///   base64 alphabet (newlines, spaces, trailing prose, etc.).
/// - Strings whose mime portion contains ASCII whitespace.
///
/// `#[inline]` because this is on the hot path of every MCP
/// content-block conversion (`From<ContentBlock>`,
/// `RichContentPart::from_text_or_data_url`, and the per-leaf
/// `file_content` extraction during log writes) and the body is
/// a handful of cheap string ops — the call overhead would be a
/// measurable fraction of the work.
#[inline]
pub fn parse_data_url(url: &str) -> Option<(&str, &str)> {
    let rest = url.strip_prefix("data:")?;
    let (mime, payload) = rest.split_once(";base64,")?;
    if mime.bytes().any(|b| b.is_ascii_whitespace()) {
        return None;
    }
    if !payload.bytes().all(is_base64_byte) {
        return None;
    }
    Some((mime, payload))
}

/// True for bytes in the standard base64 alphabet
/// (`[A-Za-z0-9+/=]`). Excludes whitespace and every URL-safe or
/// padding-variant character — data URLs are required to use the
/// standard alphabet, and rejecting anything else is exactly what
/// makes [`parse_data_url`] refuse to swallow non-data-URL content.
#[inline]
fn is_base64_byte(b: u8) -> bool {
    matches!(b, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'+' | b'/' | b'=')
}
