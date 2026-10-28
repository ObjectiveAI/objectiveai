//! Whether, and how often, a link dials again after its connection
//! ends.

use std::fmt;
use std::time::Duration;

use serde::de::{self, Deserializer, Visitor};
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};

/// How long the link waits by default before it dials again: as long as
/// the daemon waits before it dials a provider again.
pub const DEFAULT_PAUSE: Duration = Duration::from_secs(5);

/// Whether a [`Link`](super::Link) dials again by itself after its
/// connection ends or a dial fails, and how long it waits first.
///
/// On the wire of a document, the string `never` or a whole number of
/// seconds, at least `1`: `redial: never`, `redial: 5`. The default is
/// [`DEFAULT_PAUSE`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Redial {
    /// Never by itself: the first use dials, a connection that ended
    /// stays ended, and only a reset puts the link back to dial on the
    /// next use.
    Never,
    /// Dialled at start, and dialled again this long after every
    /// failed dial and every connection that ended, for as long as the
    /// link lives; a reset dials at once.
    Every(Duration),
}

impl Redial {
    /// Whether this is the default, for a document that leaves it out.
    pub fn is_default(&self) -> bool {
        *self == Redial::default()
    }
}

impl Default for Redial {
    fn default() -> Self {
        Redial::Every(DEFAULT_PAUSE)
    }
}

impl Serialize for Redial {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Redial::Never => serializer.serialize_str("never"),
            Redial::Every(pause) => serializer.serialize_u64(pause.as_secs()),
        }
    }
}

impl<'de> Deserialize<'de> for Redial {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(RedialVisitor)
    }
}

/// Reads the word or the seconds.
struct RedialVisitor;

impl Visitor<'_> for RedialVisitor {
    type Value = Redial;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("`never` or a whole number of seconds, at least 1")
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Redial, E> {
        match value {
            "never" => Ok(Redial::Never),
            other => Err(E::invalid_value(de::Unexpected::Str(other), &self)),
        }
    }

    fn visit_u64<E: de::Error>(self, seconds: u64) -> Result<Redial, E> {
        if seconds == 0 {
            return Err(E::invalid_value(de::Unexpected::Unsigned(0), &self));
        }
        Ok(Redial::Every(Duration::from_secs(seconds)))
    }

    fn visit_i64<E: de::Error>(self, seconds: i64) -> Result<Redial, E> {
        match u64::try_from(seconds) {
            Ok(seconds) => self.visit_u64(seconds),
            Err(_) => Err(E::invalid_value(de::Unexpected::Signed(seconds), &self)),
        }
    }
}
