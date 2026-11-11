//! Why a document's text would not read.

use std::fmt;

/// A `/DIVERGE.md` that is not one. Each names the rule of
/// [`Document::parse`](super::Document::parse) the text broke, in the
/// order the rules are checked.
#[derive(Debug)]
pub enum ParseError {
    /// The text does not begin with a line that is exactly `---`.
    NoFrontmatter,
    /// No later line is exactly `---`, so the frontmatter never
    /// closes.
    Unterminated,
    /// The frontmatter is not JSON, or not an object of exactly the
    /// members `name`, a string, and `schema`. The message is the
    /// parser's, which quotes no value.
    Json(serde_json::Error),
    /// The frontmatter's `schema` is not an object, which a schema is.
    NotObject,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::NoFrontmatter => f.write_str("document does not open with a `---` line"),
            ParseError::Unterminated => f.write_str("document frontmatter is never closed by a `---` line"),
            ParseError::Json(error) => write!(f, "document frontmatter is not a name and a schema: {error}"),
            ParseError::NotObject => f.write_str("document schema is not a JSON object"),
        }
    }
}

impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ParseError::Json(error) => Some(error),
            ParseError::NoFrontmatter | ParseError::Unterminated | ParseError::NotObject => None,
        }
    }
}
