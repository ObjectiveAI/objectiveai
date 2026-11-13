//! Not at all, or as `T` says.

use std::borrow::Cow;
use std::fmt;
use std::marker::PhantomData;

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::de::value::MapAccessDeserializer;
use serde::de::{self, Deserializer, MapAccess, Visitor};
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};

/// Something an image's program does not do, or does as `T` says.
///
/// On the wire, the boolean `false` or `T` as it is, an object:
/// `"chat": false`, `"chat": {…}`. The rules:
///
/// 1. `false` is [`FalseOr::False`].
/// 2. An object is read as `T`, by `T`'s own rules, and is
///    [`FalseOr::Is`].
/// 3. `true` does not decode: there is no doing it without saying how.
/// 4. Any other value — `null`, a number, a string, an array — does
///    not decode.
/// 5. The default, what a member left absent means, is
///    [`FalseOr::False`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FalseOr<T> {
    /// Not at all: the boolean `false`.
    False,
    /// As `T` says, as `T` is on the wire.
    Is(T),
}

impl<T> FalseOr<T> {
    /// Whether this is `false`: what a member left absent means.
    pub fn is_false(&self) -> bool {
        matches!(self, FalseOr::False)
    }

    /// What `T` says, where there is one.
    pub fn as_ref(&self) -> Option<&T> {
        match self {
            FalseOr::False => None,
            FalseOr::Is(is) => Some(is),
        }
    }
}

impl<T> Default for FalseOr<T> {
    /// Rule 5.
    fn default() -> Self {
        FalseOr::False
    }
}

impl<T: Serialize> Serialize for FalseOr<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            FalseOr::False => serializer.serialize_bool(false),
            FalseOr::Is(is) => is.serialize(serializer),
        }
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for FalseOr<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(FalseOrVisitor(PhantomData))
    }
}

/// Reads the `false` or the object.
struct FalseOrVisitor<T>(PhantomData<T>);

impl<'de, T: Deserialize<'de>> Visitor<'de> for FalseOrVisitor<T> {
    type Value = FalseOr<T>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("`false` or an object")
    }

    /// Rules 1 and 3.
    fn visit_bool<E: de::Error>(self, value: bool) -> Result<FalseOr<T>, E> {
        match value {
            false => Ok(FalseOr::False),
            true => Err(E::invalid_value(de::Unexpected::Bool(true), &self)),
        }
    }

    /// Rule 2.
    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<FalseOr<T>, A::Error> {
        T::deserialize(MapAccessDeserializer::new(map)).map(FalseOr::Is)
    }
}

/// The two shapes as a schema: the one boolean, or `T`'s own. Written
/// by hand because the derive follows a derived serde and this type's
/// serde is its own. `T`'s schema is an object's, so the two never
/// both hold and a value is exactly one of them.
impl<T: JsonSchema> JsonSchema for FalseOr<T> {
    fn schema_name() -> Cow<'static, str> {
        format!("FalseOr_{}", T::schema_name()).into()
    }

    fn schema_id() -> Cow<'static, str> {
        format!("diverge_sdk::shared::containers::info::FalseOr<{}>", T::schema_id()).into()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        let is = generator.subschema_for::<T>();
        json_schema!({
            "anyOf": [
                { "type": "boolean", "const": false },
                is
            ]
        })
    }
}
