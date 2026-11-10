//! Serde helpers for `rust_decimal::Decimal` that reject string inputs.
//!
//! `Decimal`'s default `Deserialize` impl accepts strings via
//! `visit_str`, which causes `#[serde(untagged)]` enums to match
//! `Decimal` variants for string values that should fall through to
//! other variants. These helpers use a `Visitor` that only accepts
//! numeric types. Ported with the types whose fields name them.

use std::fmt;

use rust_decimal::Decimal;
use serde::Deserialize;
use serde::de::{self, Deserializer, Visitor};

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

/// Deserializes a `Decimal` from numeric JSON values only (rejects
/// strings).
pub fn decimal<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Decimal, D::Error> {
    deserializer.deserialize_any(NumericDecimalVisitor)
}

/// Deserializes an `Option<Decimal>` from numeric JSON values only.
pub fn option_decimal<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Decimal>, D::Error> {
    Option::<DecimalFromNumeric>::deserialize(deserializer)
        .map(|opt| opt.map(|d| d.0))
}

/// Newtype so `Option` deserialization delegates to our numeric-only
/// visitor.
struct DecimalFromNumeric(Decimal);

impl<'de> Deserialize<'de> for DecimalFromNumeric {
    fn deserialize<D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Self, D::Error> {
        deserializer
            .deserialize_any(NumericDecimalVisitor)
            .map(DecimalFromNumeric)
    }
}
