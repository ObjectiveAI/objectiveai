//! Every agent, the dependency's own, or only these.

use std::borrow::Cow;

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::de::Deserializer;
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};

use crate::shared::permission::Tags;

/// How far a grant over agents reaches: every agent there is, the
/// dependency's own agents, or only those the [`Tags`] let through.
/// Flat on the wire: the string `"any"`, the string `"self"`, or the
/// tags object as it is. Neither word is a tags object, so the three
/// shapes never collide; a string that is neither does not decode.
///
/// # `self` is the agents the dependency is a dependency of
///
/// The one word the agents kind has that no other has, and that no
/// grant of the daemon's own has: it names nothing and reaches, for a
/// transient tool, the one agent it was deployed for, and, for a
/// service, every agent attached to it — attached by a registration
/// that declared the service's template, and still so — whether or
/// not any of them is active. A tool that may `message` within
/// `self` may message its own agents and no other, whatever their
/// tags, which is what a tool that answers to its agents asks for.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Within {
    /// Every agent: the string `"any"`.
    Any,
    /// The dependency's own agents: the string `"self"`.
    Self_,
    /// Only the agents the tags let through, as a test.
    Only(Tags),
}

impl Serialize for Within {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Within::Any => serializer.serialize_str("any"),
            Within::Self_ => serializer.serialize_str("self"),
            Within::Only(only) => only.serialize(serializer),
        }
    }
}

/// The two words, so that each is read as a word and not as tags.
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Word {
    Any,
    #[serde(rename = "self")]
    Self_,
}

/// The two shapes, tried in order: a word, then the tags.
#[derive(Deserialize)]
#[serde(untagged)]
enum Repr {
    Word(Word),
    Only(Tags),
}

impl<'de> Deserialize<'de> for Within {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(match Repr::deserialize(deserializer)? {
            Repr::Word(Word::Any) => Within::Any,
            Repr::Word(Word::Self_) => Within::Self_,
            Repr::Only(only) => Within::Only(only),
        })
    }
}

/// The three shapes as a schema: either word, or the tags' own.
/// Written by hand because the derive follows a derived serde and
/// this type's serde is its own.
impl JsonSchema for Within {
    fn schema_name() -> Cow<'static, str> {
        "AgentsWithin".into()
    }

    fn schema_id() -> Cow<'static, str> {
        "diverge_sdk::shared::permission::agents::Within".into()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        let only = generator.subschema_for::<Tags>();
        json_schema!({
            "anyOf": [
                { "type": "string", "const": "any" },
                { "type": "string", "const": "self" },
                only
            ]
        })
    }
}
