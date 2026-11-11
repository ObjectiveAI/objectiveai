//! One short line of plain text: a daemon's announcement, as a
//! provider's `daemons::list` lists it, and an image's description.

use std::borrow::Cow;
use std::fmt;

use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Serialize};

/// The most characters a [`Description`] holds: the limit, and the
/// plain-text rule with it, of the `description` of a Chrome extension's
/// manifest — "no more than 132 characters".
pub const DESCRIPTION_CHARACTERS: usize = 132;

/// One short line of plain text. It is what a daemon's announcement
/// is made of — the text of its
/// [`daemons::announce`](crate::provider::endpoints::daemons::announce),
/// which a provider's `daemons::list` lists beside its identity — and
/// what an image's description is made of. The rules:
///
/// 1. **Plain text.** One line: no control character — no newline, no
///    tab — anywhere in it.
/// 2. **Short.** At least one character, and at most
///    [`DESCRIPTION_CHARACTERS`], counted as Unicode scalar values.
/// 3. **Trimmed.** No whitespace at its start or its end, so that two
///    descriptions that read the same are the same.
///
/// A string that breaks a rule is no description: it does not
/// deserialize, and on the wire it is malformed.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Description(String);

impl Description {
    /// `text` as a description, or the rule it breaks.
    pub fn new(text: impl Into<String>) -> Result<Description, DescriptionError> {
        let text = text.into();
        Description::check(&text)?;
        Ok(Description(text))
    }

    /// Which rule `text` breaks, if any.
    pub fn check(text: &str) -> Result<(), DescriptionError> {
        let characters = text.chars().count();
        if characters == 0 {
            return Err(DescriptionError::Empty);
        }
        if characters > DESCRIPTION_CHARACTERS {
            return Err(DescriptionError::Long(characters));
        }
        if text.chars().any(char::is_control) {
            return Err(DescriptionError::Control);
        }
        if text.trim() != text {
            return Err(DescriptionError::Untrimmed);
        }
        Ok(())
    }

    /// The text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Description {
    type Error = DescriptionError;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        Description::new(text)
    }
}

impl From<Description> for String {
    fn from(description: Description) -> String {
        description.0
    }
}

impl fmt::Display for Description {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A string, of one to [`DESCRIPTION_CHARACTERS`] characters.
impl JsonSchema for Description {
    fn schema_name() -> Cow<'static, str> {
        "Description".into()
    }

    fn schema_id() -> Cow<'static, str> {
        "diverge_sdk::shared::description::Description".into()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        schemars::json_schema!({
            "type": "string",
            "minLength": 1,
            "maxLength": DESCRIPTION_CHARACTERS,
            "pattern": "^\\S(.*\\S)?$",
            "description": "One line of plain text, trimmed: what a party says of itself."
        })
    }
}

/// The rule a string broke.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DescriptionError {
    /// No characters.
    Empty,
    /// More than [`DESCRIPTION_CHARACTERS`]: how many.
    Long(usize),
    /// A control character.
    Control,
    /// Whitespace at the start or the end.
    Untrimmed,
}

impl fmt::Display for DescriptionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DescriptionError::Empty => f.write_str("a description is empty"),
            DescriptionError::Long(characters) => {
                write!(f, "a description is {characters} characters, more than {DESCRIPTION_CHARACTERS}")
            }
            DescriptionError::Control => f.write_str("a description holds a control character"),
            DescriptionError::Untrimmed => f.write_str("a description starts or ends with whitespace"),
        }
    }
}

impl std::error::Error for DescriptionError {}
