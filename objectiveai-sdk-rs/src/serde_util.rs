//! Serde helpers for `rust_decimal::Decimal` that reject string inputs.
//!
//! `Decimal`'s default `Deserialize` impl accepts strings via `visit_str`,
//! which causes `#[serde(untagged)]` enums to match `Decimal` variants for
//! string values that should fall through to other variants (e.g. `Err(Value)`).
//! These helpers use a `Visitor` that only accepts numeric types.

use rust_decimal::Decimal;
use serde::de::{self, Deserializer, SeqAccess, Visitor};
use serde::Deserialize;
use std::fmt;

struct NumericDecimalVisitor;

impl<'de> Visitor<'de> for NumericDecimalVisitor {
    type Value = Decimal;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a number")
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Decimal, E> {
        Ok(Decimal::from(v))
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Decimal, E> {
        Ok(Decimal::from(v))
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Decimal, E> {
        Decimal::try_from(v).map_err(de::Error::custom)
    }
}

/// Deserializes a `Decimal` from numeric JSON values only (rejects strings).
pub fn decimal<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Decimal, D::Error> {
    deserializer.deserialize_any(NumericDecimalVisitor)
}

/// Deserializes an `Option<Decimal>` from numeric JSON values only.
pub fn option_decimal<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<Decimal>, D::Error> {
    Option::<DecimalFromNumeric>::deserialize(deserializer).map(|opt| opt.map(|d| d.0))
}

/// Deserializes a `Vec<Decimal>` from an array of numeric JSON values only.
pub fn vec_decimal<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<Decimal>, D::Error> {
    deserializer.deserialize_seq(VecDecimalVisitor)
}

/// Deserializes a `Vec<Vec<Decimal>>` from a nested array of numeric JSON values only.
pub fn vec_vec_decimal<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<Vec<Decimal>>, D::Error> {
    deserializer.deserialize_seq(VecVecDecimalVisitor)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Newtype so `Option` deserialization delegates to our numeric-only visitor.
struct DecimalFromNumeric(Decimal);

impl<'de> Deserialize<'de> for DecimalFromNumeric {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(NumericDecimalVisitor).map(DecimalFromNumeric)
    }
}

struct VecDecimalVisitor;

impl<'de> Visitor<'de> for VecDecimalVisitor {
    type Value = Vec<Decimal>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("an array of numbers")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Vec<Decimal>, A::Error> {
        let mut v = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        while let Some(d) = seq.next_element_seed(NumericDecimalSeed)? {
            v.push(d);
        }
        Ok(v)
    }
}

struct VecVecDecimalVisitor;

impl<'de> Visitor<'de> for VecVecDecimalVisitor {
    type Value = Vec<Vec<Decimal>>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("an array of arrays of numbers")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Vec<Vec<Decimal>>, A::Error> {
        let mut v = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        while let Some(inner) = seq.next_element_seed(VecDecimalSeed)? {
            v.push(inner);
        }
        Ok(v)
    }
}

/// `DeserializeSeed` that deserializes a single `Decimal` via `NumericDecimalVisitor`.
struct NumericDecimalSeed;

impl<'de> de::DeserializeSeed<'de> for NumericDecimalSeed {
    type Value = Decimal;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Decimal, D::Error> {
        deserializer.deserialize_any(NumericDecimalVisitor)
    }
}

/// `DeserializeSeed` that deserializes a `Vec<Decimal>` via `VecDecimalVisitor`.
struct VecDecimalSeed;

impl<'de> de::DeserializeSeed<'de> for VecDecimalSeed {
    type Value = Vec<Decimal>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Vec<Decimal>, D::Error> {
        deserializer.deserialize_seq(VecDecimalVisitor)
    }
}
