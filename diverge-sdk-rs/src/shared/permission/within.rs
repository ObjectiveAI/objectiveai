//! Every one, or only these.

use std::borrow::Cow;

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::de::Deserializer;
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};

/// How far a grant reaches, or which tags it may touch: every one there
/// is, or only what `T` names. Flat on the wire: the string `"any"`, or
/// `T` as it is — a filter object, a list of tags. `"any"` is no filter
/// and no list, so the two shapes never collide; a string that is not
/// `"any"` does not decode.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Within<T> {
    /// Every one: the string `"any"`.
    Any,
    /// Only what `T` names, as `T` is on the wire.
    Only(T),
}

impl<T: Serialize> Serialize for Within<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Within::Any => serializer.serialize_str("any"),
            Within::Only(only) => only.serialize(serializer),
        }
    }
}

/// The one word, so that it is read as a word and not as a `T`.
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Word {
    Any,
}

/// The two shapes, tried in order: the word, then `T`.
#[derive(Deserialize)]
#[serde(untagged)]
enum Repr<T> {
    Word(Word),
    Only(T),
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Within<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(match Repr::deserialize(deserializer)? {
            Repr::Word(Word::Any) => Within::Any,
            Repr::Only(only) => Within::Only(only),
        })
    }
}

/// The two shapes as a schema: the one word, or `T`'s own. Written by
/// hand because the derive follows a derived serde and this type's
/// serde is its own.
impl<T: JsonSchema> JsonSchema for Within<T> {
    fn schema_name() -> Cow<'static, str> {
        format!("Within_{}", T::schema_name()).into()
    }

    fn schema_id() -> Cow<'static, str> {
        format!("diverge_sdk::shared::permission::Within<{}>", T::schema_id()).into()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        let only = generator.subschema_for::<T>();
        json_schema!({
            "anyOf": [
                { "type": "string", "const": "any" },
                only
            ]
        })
    }
}
