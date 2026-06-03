
use super::*;
use objectiveai_sdk::agent::completions::message::{File as FileBlob, ImageUrl, InputAudio};
use rmcp::model::RawContent;
use serde_json::{Value, json};

/// Construct a typed `RunItem::Command(ResponseItem)` from a JSON
/// value that matches the externally-tagged wire shape. Lets tests
/// stay short — building the full nested ResponseItem chain by
/// hand is impractical at 6+ levels deep for the Logs tier.
fn run_item(value: Value) -> Result<RunItem, Error> {
    let ri: ResponseItem = serde_json::from_value(value).expect("ResponseItem deserialize");
    Ok(RunItem::Command(ri))
}

/// `Tools::Run::Stdout(line)` shorthand.
fn stdout_line(line: &str) -> Result<RunItem, Error> {
    run_item(json!({"Tools": {"Run": line}}))
}

/// `Tools::Run::Stderr(cli::Error)` shorthand. The untagged variant
/// distinguishes Stderr from Stdout by the `"type":"error"` tag on
/// the inner cli::Error.
fn stderr_line(line: &str) -> Result<RunItem, Error> {
    run_item(json!({
        "Tools": {"Run": {"type": "error", "message": line}}
    }))
}

/// `Plugins::Run::Notification(<value>)` shorthand.
fn plugin_notification(value: Value) -> Result<RunItem, Error> {
    run_item(json!({"Plugins": {"Run": value}}))
}

fn plugin_notification_string(s: &str) -> Result<RunItem, Error> {
    plugin_notification(Value::String(s.to_string()))
}

/// A free-form CLI error. Renders as the
/// `{"type":"error","fatal":true,...}` envelope.
fn err(message: &str) -> Result<RunItem, Error> {
    Err(Error::MissingArgs(Box::leak(
        message.to_string().into_boxed_str(),
    )))
}

/// Construct a Logs-tier `Image` leaf via JSON synthesis. Picks
/// the shortest valid path — `Logs::Agents::Completions::Response::Messages::Image::Get(ImageUrl)`
/// — that exercises the chain walker.
fn image_url_leaf(url: &str) -> Value {
    json!({
        "Logs": {"Agents": {"Completions": {"Response": {"Messages": {"Image": {"Get": {"url": url}}}}}}}
    })
}

fn audio_leaf(data: &str, format: &str) -> Value {
    json!({
        "Logs": {"Agents": {"Completions": {"Response": {"Messages": {"Audio": {"Get": {"data": data, "format": format}}}}}}}
    })
}

fn video_leaf(url: &str) -> Value {
    json!({
        "Logs": {"Agents": {"Completions": {"Response": {"Messages": {"Video": {"Get": {"url": url}}}}}}}
    })
}

fn file_leaf(file_data: &str, filename: &str) -> Value {
    json!({
        "Logs": {"Agents": {"Completions": {"Response": {"Messages": {"File": {"Get": {"file_data": file_data, "filename": filename}}}}}}}
    })
}

fn text_leaf(text: &str) -> Value {
    json!({
        "Logs": {"Agents": {"Completions": {"Response": {"Messages": {"Text": {"Get": text}}}}}}
    })
}

/// Concatenate the response's text-content bodies and skip every
/// media block. The flanking `"` text blocks that bracket each
/// media block are kept; together they form an empty string element
/// (`""`) in the JSON-array view — which is what makes the
/// strip-media result still parse as a valid `Vec<String>`.
fn collect_body_strip_media(blocks: &[Content]) -> String {
    let mut s = String::new();
    for block in blocks {
        if let RawContent::Text(t) = &block.raw {
            s.push_str(&t.text);
        }
    }
    s
}

fn parse_array_of_strings(body: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(body)
        .unwrap_or_else(|e| panic!("body is not a JSON array of strings: {e}; body: {body}"))
}

#[test]
fn empty_returns_sentinel() {
    let blocks = format_items(&[]);
    assert_eq!(blocks.len(), 1);
    match &blocks[0].raw {
        RawContent::Text(t) => assert_eq!(t.text, "<empty>"),
        other => panic!("expected text block, got {other:?}"),
    }
}

#[test]
fn single_toolline_stdout_is_raw_line() {
    let items = vec![stdout_line("hello world\n")];
    let blocks = format_items(&items);
    let body = collect_body_strip_media(&blocks);
    let arr = parse_array_of_strings(&body);
    assert_eq!(arr, vec!["hello world\n"]);
}

#[test]
fn single_toolline_stderr_is_bare_error_json() {
    let items = vec![stderr_line("oops")];
    let blocks = format_items(&items);
    let body = collect_body_strip_media(&blocks);
    let arr = parse_array_of_strings(&body);
    assert_eq!(arr.len(), 1);
    let inner: Value = serde_json::from_str(&arr[0]).expect("inner is JSON");
    assert_eq!(inner["type"], "error");
    assert_eq!(inner["message"], "oops");
}

#[test]
fn plugin_notification_string_payload() {
    let items = vec![plugin_notification_string("plain text")];
    let blocks = format_items(&items);
    let body = collect_body_strip_media(&blocks);
    let arr = parse_array_of_strings(&body);
    assert_eq!(arr, vec!["plain text"]);
}

#[test]
fn plugin_notification_object_payload() {
    let items = vec![plugin_notification(json!({"hello": "world"}))];
    let blocks = format_items(&items);
    let body = collect_body_strip_media(&blocks);
    let arr = parse_array_of_strings(&body);
    assert_eq!(arr, vec![r#"{"hello":"world"}"#]);
}

#[test]
fn single_error_is_full_envelope_quoted() {
    let items = vec![err("nope")];
    let blocks = format_items(&items);
    let body = collect_body_strip_media(&blocks);
    let arr = parse_array_of_strings(&body);
    assert_eq!(arr.len(), 1);
    let inner: Value = serde_json::from_str(&arr[0]).expect("inner is JSON");
    assert_eq!(inner["type"], "error");
    assert_eq!(inner["fatal"], true);
}

#[test]
fn multi_mixed_outputs() {
    let items = vec![
        stdout_line("a"),
        stderr_line("b"),
        plugin_notification_string("c"),
    ];
    let blocks = format_items(&items);
    let body = collect_body_strip_media(&blocks);
    let arr = parse_array_of_strings(&body);
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0], "a");
    let inner1: Value = serde_json::from_str(&arr[1]).expect("inner1 is JSON");
    assert_eq!(inner1["message"], "b");
    assert_eq!(arr[2], "c");
}

#[test]
fn log_text_leaf_renders_as_string() {
    let items = vec![run_item(text_leaf("hello"))];
    let blocks = format_items(&items);
    let body = collect_body_strip_media(&blocks);
    let arr = parse_array_of_strings(&body);
    assert_eq!(arr, vec!["hello"]);
}

#[test]
fn log_image_leaf_emits_media_block() {
    let items = vec![run_item(image_url_leaf("data:image/png;base64,iVBORw0KGgo"))];
    let blocks = format_items(&items);
    assert!(
        blocks.iter().any(|b| matches!(b.raw, RawContent::Image(_))),
        "expected an Image content block"
    );
    let body = collect_body_strip_media(&blocks);
    let arr = parse_array_of_strings(&body);
    assert_eq!(arr, vec![""]);
}

#[test]
fn log_audio_leaf_emits_audio_block() {
    let items = vec![run_item(audio_leaf("SUQzBAA", "audio/mpeg"))];
    let blocks = format_items(&items);
    assert!(
        blocks.iter().any(|b| matches!(b.raw, RawContent::Audio(_))),
        "expected an Audio content block"
    );
    let body = collect_body_strip_media(&blocks);
    let arr = parse_array_of_strings(&body);
    assert_eq!(arr, vec![""]);
}

/// VideoUrl/InputVideo go through the SDK's RichContentPart →
/// ContentBlock conversion as a Text carrier (the SDK never emits
/// EmbeddedResource for video by design). From the strip-media
/// projection, the data URL survives as a single string element.
#[test]
fn log_video_leaf_lands_as_text_carrier() {
    let items = vec![run_item(video_leaf("data:video/mp4;base64,AAAA"))];
    let blocks = format_items(&items);
    let body = collect_body_strip_media(&blocks);
    let arr = parse_array_of_strings(&body);
    assert_eq!(arr, vec!["data:video/mp4;base64,AAAA"]);
}

/// File similarly lands as a Text carrier — the SDK encodes file
/// payloads as a `data:application/octet-stream;base64,...` URL
/// with marker meta on the conversion side.
#[test]
fn log_file_leaf_lands_as_text_carrier() {
    let items = vec![run_item(file_leaf("JVBERi0", "report.pdf"))];
    let blocks = format_items(&items);
    let body = collect_body_strip_media(&blocks);
    let arr = parse_array_of_strings(&body);
    assert_eq!(arr.len(), 1);
    assert!(
        arr[0].starts_with("data:application/octet-stream;base64,"),
        "expected file_data data URL, got {}",
        arr[0]
    );
}

#[test]
fn mixed_text_and_media_concat_parses() {
    let items = vec![
        stdout_line("before"),
        run_item(image_url_leaf("data:image/png;base64,iVBORw0KGgo")),
        stderr_line("after"),
    ];
    let blocks = format_items(&items);
    let body = collect_body_strip_media(&blocks);
    let arr = parse_array_of_strings(&body);
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0], "before");
    assert_eq!(arr[1], "");
    let inner: Value = serde_json::from_str(&arr[2]).expect("inner is JSON");
    assert_eq!(inner["message"], "after");
}

#[test]
fn special_chars_escape_cleanly() {
    let items = vec![stdout_line(r#"with "quotes" and \backslash"#)];
    let blocks = format_items(&items);
    let body = collect_body_strip_media(&blocks);
    let arr = parse_array_of_strings(&body);
    assert_eq!(arr, vec![r#"with "quotes" and \backslash"#]);
}

// ───────────────────────────────────────────────────────────────
// Mangle-and-length round-trip tests.
//
// Each test builds a `Vec<Result<RunItem, Error>>` whose i-th
// entry is paired with a known *expected string* at index i in the
// strip-media JSON-array view: string-carrier items expect their
// raw payload, media items expect `""` (the empty quoted-string
// slot the `"`-image-`"` flanking pattern collapses to when the
// media block is removed). The harness round-trips through
// `format_items` and asserts (a) the body parses as `Vec<String>`,
// (b) every element matches by index, (c) the expected number of
// `RawContent::Image` blocks survives.
// ───────────────────────────────────────────────────────────────

fn valid_png_data_url() -> &'static str {
    "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII="
}

fn image_item() -> Result<RunItem, Error> {
    run_item(image_url_leaf(valid_png_data_url()))
}

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
    items: &[Result<RunItem, Error>],
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
    let mut items: Vec<Result<RunItem, Error>> = Vec::with_capacity(corpus.len() * 4);
    for s in &corpus {
        items.push(stdout_line(s));
        items.push(image_item());
        items.push(plugin_notification_string(s));
        items.push(image_item());
    }
    let expected: Vec<&str> = corpus
        .iter()
        .flat_map(|s| [s.as_str(), "", s.as_str(), ""])
        .collect();
    let expected_images = corpus.len() * 2;
    assert_strings_survive(&items, &expected, expected_images);
}

#[test]
fn back_to_back_images_between_strings_survive_roundtrip() {
    let items = vec![
        stdout_line("before"),
        image_item(),
        image_item(),
        image_item(),
        stdout_line("\"between\" \\quotes\\"),
        image_item(),
        plugin_notification_string("end\nwith\nnewlines"),
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
    assert_strings_survive(&items, &expected, 4);
}

#[test]
fn extreme_length_with_dense_quotes_and_image_survives() {
    let unit = "\"adv\" \\seg\\ \n\t mix ✓✗ ";
    let big = unit.repeat(2200);
    assert!(big.len() > 50 * 1024, "big string too small: {}", big.len());

    let items = vec![stdout_line(&big), image_item(), stdout_line("tail")];
    let blocks = format_items(&items);
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

/// Suppress the unused-import warning that the test corpus's
/// borrowed-but-not-constructed types would otherwise raise on a
/// debug build (FileBlob / InputAudio / ImageUrl are used via
/// `serde_json::from_value` paths inside the formatter, not at the
/// test layer).
#[allow(dead_code)]
fn _silence_unused(_: FileBlob, _: InputAudio, _: ImageUrl) {}