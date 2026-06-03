//! Streaming JSON-array formatter for MCP tool responses.
//!
//! Consumes the CLI's typed `Result<RunItem, Error>` stream and emits
//! MCP `Content` blocks shaped so the concatenated text view of the
//! response parses as a `Vec<String>` JSON array, with media blocks
//! flanked by `"` text blocks so the strip-media projection is still
//! valid JSON.
//!
//! # Pipeline
//!
//! Each stream item projects to one `RichContentPart` via the dispatch
//! table in [`item_to_rich_content_part`]. String-shaped parts route to
//! a single quoted-string text block; media parts route through the
//! SDK's `ContentBlock` → bridge → `rmcp::Content`, flanked by `"`
//! blocks so the array shape survives the media interleave.
//!
//! # Dispatch table
//!
//! | Stream item | Rendered payload |
//! |-------------|------------------|
//! | `Err(cli error)` | `cli::Error` envelope JSON (matches `main.rs::write_error_line` shape) |
//! | `Tools::Run::Stdout(line)` | raw line string |
//! | `Tools::Run::Stderr(cli::Error)` | full `cli::Error` JSON |
//! | `Plugins::Run::Notification(Value::String(s))` | raw string |
//! | `Plugins::Run::Notification(other)` | JSON of the value |
//! | `Plugins::Run::Typed(typed)` | JSON of `ResponseTyped` |
//! | `Logs` tier — media leaf | `RichContentPart::{ImageUrl,InputAudio,VideoUrl,File}` |
//! | `Logs` tier — string leaf | raw string |
//! | `Logs` tier — other leaf | JSON of the leaf payload |
//! | everything else | JSON of the full `RunItem` (externally-tagged path included) |
//!
//! Media detection for the Logs tier walks the externally-tagged
//! aggregator chain looking for an `Image` / `Audio` / `Video` / `File`
//! variant name (the discriminator between e.g. ImageUrl and VideoUrl
//! lives in the aggregator path, not in the leaf payload, since
//! ImageUrl and VideoUrl both serialize as `{"url": "…"}`). Once
//! identified, the inner payload is deserialized as the matching
//! media type.

use objectiveai_cli::RunItem;
use objectiveai_cli::error::Error;
use objectiveai_sdk::agent::completions::message::{
    File, ImageUrl, InputAudio, RichContentPart, VideoUrl,
};
use objectiveai_sdk::cli::command::ResponseItem;
use objectiveai_sdk::cli::command::plugins::ResponseItem as PluginsResponseItem;
use objectiveai_sdk::cli::command::plugins::run as plugins_run;
use objectiveai_sdk::cli::command::tools::ResponseItem as ToolsResponseItem;
use objectiveai_sdk::cli::command::tools::run as tools_run;
use objectiveai_sdk::mcp::tool::ContentBlock;
use rmcp::model::Content;
use serde_json::Value;

use crate::bridge::into_rmcp_content;

/// Sentinel returned for `items.is_empty()`. An MCP client reading
/// the response as text gets `<empty>` rather than `[]`.
const EMPTY_SENTINEL: &str = "<empty>";

/// Format the CLI's collected `Result<RunItem, Error>` stream items
/// into the MCP tool response `Vec<Content>`. See the module-level
/// docs for the dispatch table and the JSON-array-of-strings
/// invariant.
pub fn format_items(items: &[Result<RunItem, Error>]) -> Vec<Content> {
    if items.is_empty() {
        return vec![Content::text(EMPTY_SENTINEL)];
    }

    // Capacity heuristic: most items emit one block; media emit three.
    let mut blocks: Vec<Content> = Vec::with_capacity(items.len() * 3 + 2);
    blocks.push(Content::text("["));
    let mut first = true;
    for item in items {
        if !first {
            blocks.push(Content::text(", "));
        }
        first = false;
        let part = item_to_rich_content_part(item);
        push_part(part, &mut blocks);
    }
    blocks.push(Content::text("]"));
    blocks
}

/// Project one stream item to one `RichContentPart`.
fn item_to_rich_content_part(item: &Result<RunItem, Error>) -> RichContentPart {
    match item {
        Err(e) => {
            let body = render_error_envelope(e);
            RichContentPart::from_text_or_data_url(body)
        }
        Ok(RunItem::Command(ri)) => response_item_to_rich_content_part(ri),
        Ok(RunItem::Instance(emission)) => {
            let body = serde_json::to_string(emission)
                .unwrap_or_else(|_| String::from("<serialize error>"));
            RichContentPart::from_text_or_data_url(body)
        }
    }
}

/// Render a CLI error as the same JSON envelope `main.rs::write_error_line`
/// emits: `{"type":"error","level":"error","fatal":true,"message":<msg>}`.
fn render_error_envelope(e: &Error) -> String {
    let payload = objectiveai_sdk::cli::Error {
        r#type: objectiveai_sdk::cli::ErrorType::Error,
        level: Some(objectiveai_sdk::cli::Level::Error),
        fatal: Some(true),
        message: e.output_message(),
    };
    serde_json::to_string(&payload).unwrap_or_else(|_| String::from("<serialize error>"))
}

/// Dispatch a `ResponseItem` to its `RichContentPart`. The Logs tier
/// goes through the chain walker for media detection; everything else
/// pattern-matches the specific leaves the legacy formatter handled.
fn response_item_to_rich_content_part(ri: &ResponseItem) -> RichContentPart {
    match ri {
        // Tools `run` — stdout line as raw text.
        ResponseItem::Tools(ToolsResponseItem::Run(tools_run::ResponseItem::Stdout(line))) => {
            RichContentPart::from_text_or_data_url(line.clone())
        }
        // Tools `run` — stderr as the full `cli::Error` envelope.
        ResponseItem::Tools(ToolsResponseItem::Run(tools_run::ResponseItem::Stderr(err))) => {
            let body = serde_json::to_string(err)
                .unwrap_or_else(|_| String::from("<serialize error>"));
            RichContentPart::from_text_or_data_url(body)
        }
        // Plugins `run` notification: string payload renders as raw
        // text; any other JSON shape renders as its JSON encoding.
        ResponseItem::Plugins(PluginsResponseItem::Run(
            plugins_run::ResponseItem::Notification(value),
        )) => match value {
            Value::String(s) => RichContentPart::from_text_or_data_url(s.clone()),
            other => {
                let body = serde_json::to_string(other)
                    .unwrap_or_else(|_| String::from("<serialize error>"));
                RichContentPart::from_text_or_data_url(body)
            }
        },
        // Plugins `run` typed events (Command / Mcp) — encode the
        // typed value verbatim so the MCP consumer sees the same
        // `{"type":"<kind>", ...}` shape the CLI wire format uses.
        ResponseItem::Plugins(PluginsResponseItem::Run(plugins_run::ResponseItem::Typed(typed))) => {
            let body = serde_json::to_string(typed)
                .unwrap_or_else(|_| String::from("<serialize error>"));
            RichContentPart::from_text_or_data_url(body)
        }
        // Plugins `run` error — `cli::Error` has its own `type:"error"`,
        // so we encode it directly (the SDK ResponseItem moved Error
        // out of the `tag=type` ResponseTyped to avoid double-typing).
        ResponseItem::Plugins(PluginsResponseItem::Run(plugins_run::ResponseItem::Error(err))) => {
            let body = serde_json::to_string(err)
                .unwrap_or_else(|_| String::from("<serialize error>"));
            RichContentPart::from_text_or_data_url(body)
        }
        // Logs tier — many leaves yield media payloads (ImageUrl,
        // InputAudio, VideoUrl, File) under multiple aggregator
        // levels. The chain walker detects the media variant name and
        // pulls out the typed payload; non-media leaves fall through
        // to text or JSON.
        ResponseItem::Logs(_) => logs_to_rich_content_part(ri),
        // Everything else: encode the full aggregator wire shape.
        // Matches what the binary executor emits for this leaf, so an
        // MCP consumer parsing the strip-media JSON array sees the
        // same line shape an `objectiveai …` stdout reader would.
        other => {
            let body = serde_json::to_string(other)
                .unwrap_or_else(|_| String::from("<serialize error>"));
            RichContentPart::from_text_or_data_url(body)
        }
    }
}

/// Walk a `Logs` tier `ResponseItem` to its leaf, identifying media by
/// the aggregator variant name (`Image` / `Audio` / `Video` / `File`)
/// rather than by leaf payload shape — since `ImageUrl` and `VideoUrl`
/// both serialize as `{"url":"…"}`, only the discriminator in the
/// containing variant tells them apart. Non-media leaves fall back to
/// text or JSON.
fn logs_to_rich_content_part(ri: &ResponseItem) -> RichContentPart {
    let Ok(value) = serde_json::to_value(ri) else {
        return RichContentPart::from_text_or_data_url(String::from("<serialize error>"));
    };
    walk_logs_chain(&value)
}

fn walk_logs_chain(value: &Value) -> RichContentPart {
    let mut current = value;
    loop {
        match current {
            Value::Object(map) if map.len() == 1 => {
                let (key, inner) = map.iter().next().unwrap();
                match key.as_str() {
                    "Image" => return media_payload_to_rich(inner, Media::Image),
                    "Audio" => return media_payload_to_rich(inner, Media::Audio),
                    "Video" => return media_payload_to_rich(inner, Media::Video),
                    "File" => return media_payload_to_rich(inner, Media::File),
                    _ => current = inner,
                }
            }
            Value::String(s) => {
                return RichContentPart::from_text_or_data_url(s.clone());
            }
            other => {
                let body = serde_json::to_string(other)
                    .unwrap_or_else(|_| String::from("<serialize error>"));
                return RichContentPart::from_text_or_data_url(body);
            }
        }
    }
}

#[derive(Clone, Copy)]
enum Media {
    Image,
    Audio,
    Video,
    File,
}

/// `value` is the JSON sitting under an `Image`/`Audio`/`Video`/`File`
/// aggregator variant. Peel any remaining PascalCase-keyed aggregator
/// wrappers (e.g. `{"Get": ...}` for the leaf's `Get` variant), then
/// deserialize as the matching SDK media type. On failure, fall back
/// to a JSON encoding of the leaf.
///
/// PascalCase-only peeling is load-bearing: SDK media payloads have
/// snake_case field names (`url`, `file_data`, `data`, …), so once we
/// hit a snake_case-keyed object we've reached the typed payload and
/// must stop. A blanket single-key-object peel would over-peel through
/// `{"url": "…"}` into the bare URL string, and worse: `File` has all
/// optional fields, so a peel-then-try loop would accept the wrong
/// layer (`{"Get": {…}}` → `File { all None }` via serde's
/// ignore-unknown-fields default).
fn media_payload_to_rich(value: &Value, kind: Media) -> RichContentPart {
    let leaf = peel_pascalcase_layers(value.clone());
    let attempted = match kind {
        Media::Image => serde_json::from_value::<ImageUrl>(leaf.clone())
            .map(|image_url| RichContentPart::ImageUrl { image_url }),
        Media::Audio => serde_json::from_value::<InputAudio>(leaf.clone())
            .map(|input_audio| RichContentPart::InputAudio { input_audio }),
        Media::Video => serde_json::from_value::<VideoUrl>(leaf.clone())
            .map(|video_url| RichContentPart::VideoUrl { video_url }),
        Media::File => serde_json::from_value::<File>(leaf.clone())
            .map(|file| RichContentPart::File { file }),
    };
    attempted.unwrap_or_else(|_| {
        let body =
            serde_json::to_string(&leaf).unwrap_or_else(|_| String::from("<serialize error>"));
        RichContentPart::from_text_or_data_url(body)
    })
}

/// Peel single-key-object wrappings whose key starts with an
/// uppercase ASCII letter (an externally-tagged enum variant name).
/// Stops at snake_case-keyed objects (the typed leaf payload) and at
/// non-object values.
fn peel_pascalcase_layers(mut value: Value) -> Value {
    loop {
        match value {
            Value::Object(map) if map.len() == 1 => {
                let first_char = map
                    .keys()
                    .next()
                    .and_then(|k| k.chars().next())
                    .unwrap_or('\0');
                if !first_char.is_ascii_uppercase() {
                    return Value::Object(map);
                }
                value = map.into_iter().next().unwrap().1;
            }
            other => return other,
        }
    }
}

/// Push one logical array element to `blocks`. `Text` parts emit a
/// single quoted text block; every other variant routes through the
/// SDK's `ContentBlock` and the bridge, flanked by `"` blocks so
/// the strip-media view of the response still parses as a JSON
/// array of strings.
fn push_part(part: RichContentPart, blocks: &mut Vec<Content>) {
    match part {
        RichContentPart::Text { text } => {
            blocks.push(quoted_text_block(&text));
        }
        other => {
            let cb: ContentBlock = other.into();
            // If the SDK forward-conversion produced a Text carrier
            // (remote ImageUrl, video, file_url, file_id, etc.), the
            // _meta markers ride along — but for the formatter's
            // JSON-array-of-strings invariant we just need the text
            // body, properly quoted. Route Text-carrier results
            // through the same quoting path as the plain Text part.
            match cb {
                ContentBlock::Text(t) => {
                    blocks.push(quoted_text_block(&t.text));
                }
                rich => {
                    blocks.push(Content::text("\""));
                    blocks.push(into_rmcp_content(rich));
                    blocks.push(Content::text("\""));
                }
            }
        }
    }
}

/// `Content::text(format!("\"{}\"", json_escape::escape_str(s)))` —
/// build a JSON-string-literal block from a raw string.
fn quoted_text_block(s: &str) -> Content {
    Content::text(format!("\"{}\"", json_escape::escape_str(s)))
}

#[cfg(test)]
mod tests {
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
}
