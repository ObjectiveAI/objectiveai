//! The image's document, parsed and written.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::ParseError;

/// `/DIVERGE.md`, parsed: the arguments' JSON Schema and the text
/// beneath it.
///
/// The frontmatter is the schema and nothing else — one required
/// field that is the whole document, as raw JSON, which is YAML's flow
/// style and so reads as a frontmatter anywhere one is expected. The
/// body is the image's prose: what it runs and the secrets it
/// declares, kept as the author left it. The provider carries and
/// does not read either; a caller reads the schema to learn what an
/// image's program takes without knowing the image.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Document {
    /// The JSON Schema of the image's
    /// [`arguments`](crate::shared::containers::request::Container::arguments),
    /// as the frontmatter states it: a JSON object, since a schema is
    /// one and [`parse`](Self::parse) refuses anything else. A
    /// [`Value`] rather than a typed schema, because a schema is a
    /// document in its own vocabulary and this crate has no business
    /// restating it.
    pub schema: Value,
    /// Everything after the closing `---` line, verbatim: leading
    /// blank lines, trailing newline and all.
    pub body: String,
}

/// Whether one line, as [`str::split_inclusive`] on `\n` yields it, is
/// a fence: exactly `---`, with or without the newline that ends it.
fn is_fence(line: &str) -> bool {
    line.strip_suffix('\n').unwrap_or(line) == "---"
}

impl Document {
    /// Read a `/DIVERGE.md` from its text.
    ///
    /// The rules, in the order they are checked:
    ///
    /// 1. The text begins with a line that is exactly `---`. A line
    ///    ends at `\n`, so a fence followed by anything else — a
    ///    space, a carriage return — is not one. Otherwise
    ///    [`ParseError::NoFrontmatter`].
    /// 2. The frontmatter is every line after it up to the next line
    ///    that is exactly `---`, which closes it. No such line is
    ///    [`ParseError::Unterminated`].
    /// 3. The frontmatter parses as JSON — otherwise
    ///    [`ParseError::Json`] — and is an object, since a JSON Schema
    ///    is one; any other value is [`ParseError::NotObject`].
    /// 4. The body is everything after the closing line, as the author
    ///    left it: nothing is trimmed, and a document whose closing
    ///    line is its last has an empty body.
    pub fn parse(text: &str) -> Result<Document, ParseError> {
        let mut lines = text.split_inclusive('\n');
        let opening = lines.next().ok_or(ParseError::NoFrontmatter)?;
        if !is_fence(opening) {
            return Err(ParseError::NoFrontmatter);
        }
        let start = opening.len();
        let mut end = start;
        loop {
            let line = lines.next().ok_or(ParseError::Unterminated)?;
            if is_fence(line) {
                let schema: Value = serde_json::from_str(&text[start..end]).map_err(ParseError::Json)?;
                if !schema.is_object() {
                    return Err(ParseError::NotObject);
                }
                let body = text[end + line.len()..].to_owned();
                return Ok(Document { schema, body });
            }
            end += line.len();
        }
    }

    /// Write the document back as its text: `---`, the schema
    /// pretty-printed, `---`, and the body, each fence on a line of its
    /// own. One shape is written and read: the assembler that makes an
    /// image's file and any later writer use this, and
    /// [`parse`](Self::parse) of what it writes is this document
    /// again, member for member, for any document whose schema is an
    /// object — which every parsed one is: a pretty-printed object has
    /// no line that is exactly `---`, and the body is copied verbatim.
    ///
    /// The `Result` is serde's: serializing a [`Value`] does not fail,
    /// and the signature carries what the serializer says rather than
    /// asserting it.
    pub fn render(&self) -> Result<String, serde_json::Error> {
        let schema = serde_json::to_string_pretty(&self.schema)?;
        Ok(format!("---\n{schema}\n---\n{}", self.body))
    }
}
