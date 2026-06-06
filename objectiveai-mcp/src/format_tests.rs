use crate::format::format_items;
use objectiveai_sdk::agent::completions::message::{
    File as FileBlob, ImageUrl, InputAudio, VideoUrl,
};
use objectiveai_sdk::cli::command::McpResponseItem;
use rmcp::model::{Content, RawContent};
use serde_json::{Value, json};

/// `McpResponseItem::JSONL(Value::String)` shorthand — a raw string
/// item, e.g. a tool stdout line or a plugin notification string.
fn jsonl_str(s: &str) -> McpResponseItem {
    McpResponseItem::JSONL(Value::String(s.to_string()))
}

/// `McpResponseItem::JSONL(<typed value>)` shorthand — e.g. an error
/// envelope or a typed log object.
fn jsonl(value: Value) -> McpResponseItem {
    McpResponseItem::JSONL(value)
}

fn valid_png_data_url() -> &'static str {
    "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII="
}

/// A real media item — a data-URL `ImageUrl` converts to
/// `ContentBlock::Image`, which the bridge renders as
/// `RawContent::Image`.
fn image_item() -> McpResponseItem {
    McpResponseItem::Media(
        ImageUrl {
            url: valid_png_data_url().to_string(),
            detail: None,
        }
        .into(),
    )
}

/// A real media item — `InputAudio` converts to `ContentBlock::Audio`.
fn audio_item(data: &str, format: &str) -> McpResponseItem {
    McpResponseItem::Media(
        InputAudio {
            data: data.to_string(),
            format: format.to_string(),
        }
        .into(),
    )
}

/// A text-carrier media item — the SDK encodes `VideoUrl` as a
/// `ContentBlock::Text` whose body is the URL (never EmbeddedResource
/// by design).
fn video_item(url: &str) -> McpResponseItem {
    McpResponseItem::Media(
        VideoUrl {
            url: url.to_string(),
        }
        .into(),
    )
}

/// A text-carrier media item — `File.file_data` encodes as a
/// `ContentBlock::Text` with a `data:application/octet-stream` URL.
fn file_item(file_data: &str, filename: &str) -> McpResponseItem {
    McpResponseItem::Media(
        FileBlob {
            file_data: Some(file_data.to_string()),
            file_url: None,
            file_id: None,
            filename: Some(filename.to_string()),
        }
        .into(),
    )
}

/// Concatenate the response's text-content bodies and skip every
/// media block. The flanking `"` text blocks that bracket each
/// media block are kept; together they form an empty string element
/// (`""`) in the JSON-array view — which is what makes the
/// strip-media result still parse as a valid array.
fn collect_body_strip_media(blocks: &[Content]) -> String {
    let mut s = String::new();
    for block in blocks {
        if let RawContent::Text(t) = &block.raw {
            s.push_str(&t.text);
        }
    }
    s
}

/// Multi-item bodies parse as `Vec<Value>` — string items and media
/// slots land as string elements, typed JSON items as their own
/// (object / array / number / bool / null) elements.
fn parse_array(body: &str) -> Vec<Value> {
    serde_json::from_str::<Vec<Value>>(body)
        .unwrap_or_else(|e| panic!("body is not a JSON array: {e}; body: {body}"))
}

fn parse_array_of_strings(body: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(body)
        .unwrap_or_else(|e| panic!("body is not a JSON array of strings: {e}; body: {body}"))
}

/// The single text block's body (panics on media / multi-block).
fn single_text_body(blocks: &[Content]) -> String {
    assert_eq!(blocks.len(), 1, "expected exactly one block");
    match &blocks[0].raw {
        RawContent::Text(t) => t.text.clone(),
        other => panic!("expected text block, got {other:?}"),
    }
}

#[test]
fn empty_returns_sentinel() {
    let blocks = format_items(vec![]);
    assert_eq!(single_text_body(&blocks), "<empty>");
}

#[test]
fn single_string_is_raw_unescaped() {
    // Single-item mode: the response IS the item — no array
    // brackets, no quotes, no escaping.
    let raw = "hello \"quoted\" world\nwith newline";
    let blocks = format_items(vec![jsonl_str(raw)]);
    assert_eq!(single_text_body(&blocks), raw);
}

#[test]
fn single_json_object_is_serialized() {
    let blocks = format_items(vec![jsonl(json!({"type": "error", "message": "oops"}))]);
    let body = single_text_body(&blocks);
    let inner: Value = serde_json::from_str(&body).expect("body is JSON");
    assert_eq!(inner["type"], "error");
    assert_eq!(inner["message"], "oops");
}

#[test]
fn single_image_is_bare_media_block() {
    let blocks = format_items(vec![image_item()]);
    assert_eq!(blocks.len(), 1);
    assert!(
        matches!(blocks[0].raw, RawContent::Image(_)),
        "expected an Image content block"
    );
}

#[test]
fn single_audio_is_bare_media_block() {
    let blocks = format_items(vec![audio_item("SUQzBAA", "audio/mpeg")]);
    assert_eq!(blocks.len(), 1);
    assert!(
        matches!(blocks[0].raw, RawContent::Audio(_)),
        "expected an Audio content block"
    );
}

#[test]
fn single_video_text_carrier_is_raw_body() {
    // VideoUrl is a Text-carrier ContentBlock; single-item mode emits
    // the body raw — the data URL itself.
    let blocks = format_items(vec![video_item("data:video/mp4;base64,AAAA")]);
    assert_eq!(single_text_body(&blocks), "data:video/mp4;base64,AAAA");
}

#[test]
fn single_file_text_carrier_is_raw_body() {
    let blocks = format_items(vec![file_item("JVBERi0", "report.pdf")]);
    let body = single_text_body(&blocks);
    assert!(
        body.starts_with("data:application/octet-stream;base64,"),
        "expected file_data data URL, got {body}"
    );
}

#[test]
fn multi_strings_parse_as_array() {
    let blocks = format_items(vec![jsonl_str("a"), jsonl_str("b"), jsonl_str("c")]);
    let body = collect_body_strip_media(&blocks);
    let arr = parse_array_of_strings(&body);
    assert_eq!(arr, vec!["a", "b", "c"]);
}

#[test]
fn multi_mixed_strings_and_objects() {
    let blocks = format_items(vec![
        jsonl_str("a"),
        jsonl(json!({"type": "error", "message": "b"})),
        jsonl_str("c"),
    ]);
    let body = collect_body_strip_media(&blocks);
    let arr = parse_array(&body);
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0], "a");
    assert_eq!(arr[1]["message"], "b");
    assert_eq!(arr[2], "c");
}

#[test]
fn multi_image_collapses_to_empty_string_slot() {
    let blocks = format_items(vec![jsonl_str("before"), image_item(), jsonl_str("after")]);
    assert!(
        blocks.iter().any(|b| matches!(b.raw, RawContent::Image(_))),
        "expected an Image content block"
    );
    let body = collect_body_strip_media(&blocks);
    let arr = parse_array_of_strings(&body);
    assert_eq!(arr, vec!["before", "", "after"]);
}

#[test]
fn multi_audio_collapses_to_empty_string_slot() {
    let blocks = format_items(vec![
        jsonl_str("x"),
        audio_item("SUQzBAA", "audio/mpeg"),
    ]);
    assert!(
        blocks.iter().any(|b| matches!(b.raw, RawContent::Audio(_))),
        "expected an Audio content block"
    );
    let body = collect_body_strip_media(&blocks);
    let arr = parse_array_of_strings(&body);
    assert_eq!(arr, vec!["x", ""]);
}

#[test]
fn multi_video_text_carrier_lands_as_string_element() {
    // Text-carrier media bodies are NOT json-escaped — a data URL
    // contains no quotes, so the element stays parseable.
    let blocks = format_items(vec![
        jsonl_str("x"),
        video_item("data:video/mp4;base64,AAAA"),
    ]);
    let body = collect_body_strip_media(&blocks);
    let arr = parse_array_of_strings(&body);
    assert_eq!(arr, vec!["x", "data:video/mp4;base64,AAAA"]);
}

#[test]
fn multi_file_text_carrier_lands_as_string_element() {
    let blocks = format_items(vec![jsonl_str("x"), file_item("JVBERi0", "report.pdf")]);
    let body = collect_body_strip_media(&blocks);
    let arr = parse_array_of_strings(&body);
    assert_eq!(arr.len(), 2);
    assert!(
        arr[1].starts_with("data:application/octet-stream;base64,"),
        "expected file_data data URL, got {}",
        arr[1]
    );
}

#[test]
fn special_chars_escape_cleanly() {
    let raw = r#"with "quotes" and \backslash"#;
    let blocks = format_items(vec![jsonl_str(raw), jsonl_str("tail")]);
    let body = collect_body_strip_media(&blocks);
    let arr = parse_array_of_strings(&body);
    assert_eq!(arr, vec![raw, "tail"]);
}

// ───────────────────────────────────────────────────────────────
// Mangle-and-length round-trip tests.
//
// Each test builds a `Vec<McpResponseItem>` whose i-th entry is
// paired with a known *expected string* at index i in the
// strip-media JSON-array view: string items expect their raw
// payload, real-media items expect `""` (the empty quoted-string
// slot the `"`-image-`"` flanking pattern collapses to when the
// media block is removed). The harness round-trips through
// `format_items` and asserts (a) the body parses as `Vec<String>`,
// (b) every element matches by index, (c) the expected number of
// `RawContent::Image` blocks survives.
// ───────────────────────────────────────────────────────────────

fn tricky_corpus() -> Vec<String> {
    vec![
        "plain".to_string(),
        "with \"embedded quotes\"".to_string(),
        "with\nreal\nnewlines".to_string(),
        "with\treal\ttabs".to_string(),
        "with \\backslash\\ pairs".to_string(),
        "mixed \"quotes\" + \\esc + \nnewline + \tend".to_string(),
        "control \x07 bell + \x1b ESC + \x00 nul".to_string(),
        "unicode ✓ ✗ → ← 漢字 🦀".to_string(),
        "{\"json\":\"in a string\",\"nested\":[1,2,3]}".to_string(),
        "\"\"".to_string(),
        String::new(),
        "x".repeat(8 * 1024),
    ]
}

fn assert_strings_survive(
    items: Vec<McpResponseItem>,
    expected: &[&str],
    expected_image_count: usize,
) {
    let blocks = format_items(items);
    let body = collect_body_strip_media(&blocks);
    let arr = parse_array_of_strings(&body);
    assert_eq!(
        arr.len(),
        expected.len(),
        "array length mismatch: got {} elements, expected {}",
        arr.len(),
        expected.len()
    );
    for (i, (got, exp)) in arr.iter().zip(expected.iter()).enumerate() {
        assert_eq!(
            got, exp,
            "mismatch at index {i}: got {got:?}, expected {exp:?}"
        );
    }
    let image_count = blocks
        .iter()
        .filter(|b| matches!(b.raw, RawContent::Image(_)))
        .count();
    assert_eq!(
        image_count, expected_image_count,
        "image block count mismatch"
    );
}

#[test]
fn tricky_strings_survive_roundtrip_with_images_between() {
    let corpus = tricky_corpus();
    let mut items: Vec<McpResponseItem> = Vec::with_capacity(corpus.len() * 4);
    for s in &corpus {
        items.push(jsonl_str(s));
        items.push(image_item());
        items.push(jsonl_str(s));
        items.push(image_item());
    }
    let expected: Vec<&str> = corpus
        .iter()
        .flat_map(|s| [s.as_str(), "", s.as_str(), ""])
        .collect();
    let expected_images = corpus.len() * 2;
    assert_strings_survive(items, &expected, expected_images);
}

#[test]
fn back_to_back_images_between_strings_survive_roundtrip() {
    let items = vec![
        jsonl_str("before"),
        image_item(),
        image_item(),
        image_item(),
        jsonl_str("\"between\" \\quotes\\"),
        image_item(),
        jsonl_str("end\nwith\nnewlines"),
    ];
    let expected: Vec<&str> = vec![
        "before",
        "",
        "",
        "",
        "\"between\" \\quotes\\",
        "",
        "end\nwith\nnewlines",
    ];
    assert_strings_survive(items, &expected, 4);
}

#[test]
fn extreme_length_with_dense_quotes_and_image_survives() {
    let unit = "\"adv\" \\seg\\ \n\t mix ✓✗ ";
    let big = unit.repeat(2200);
    assert!(big.len() > 50 * 1024, "big string too small: {}", big.len());

    let items = vec![jsonl_str(&big), image_item(), jsonl_str("tail")];
    let blocks = format_items(items);
    let body = collect_body_strip_media(&blocks);
    let arr = parse_array_of_strings(&body);

    assert_eq!(arr.len(), 3, "expected 3 array elements");
    assert_eq!(
        arr[0].as_bytes(),
        big.as_bytes(),
        "big string differs by {} bytes at length {}",
        arr[0]
            .as_bytes()
            .iter()
            .zip(big.as_bytes())
            .filter(|(a, b)| a != b)
            .count(),
        arr[0].len(),
    );
    assert_eq!(arr[1], "", "image slot must be empty string");
    assert_eq!(arr[2], "tail");

    let image_count = blocks
        .iter()
        .filter(|b| matches!(b.raw, RawContent::Image(_)))
        .count();
    assert_eq!(image_count, 1, "exactly one image block expected");
}
