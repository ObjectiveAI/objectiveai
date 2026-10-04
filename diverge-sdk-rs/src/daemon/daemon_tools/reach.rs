//! How far one of the daemon's tools reaches: not at all, everything,
//! or only what is named.

use serde::de::Deserializer;
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};

use super::word::Word;

/// The reach of one of the daemon's tools, as the member of a
/// create's [`Inner`](crate::daemon::create::Inner) states it. On the wire the
/// string `"disabled"`, the string `"any"`, or `T` itself, flat —
/// a filter, a pair of filters, or a list of ids, as the member
/// says — with nothing wrapped around it. `Disabled` is the default,
/// and what a member left out decodes as.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub enum Reach<T> {
    /// The container does not hold the tool.
    #[default]
    Disabled,
    /// The container holds the tool, and it reaches every thing of
    /// the caller's.
    Any,
    /// The container holds the tool, and it reaches only what `T`
    /// names.
    Only(T),
}

impl<T: Serialize> Serialize for Reach<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Reach::Disabled => Word::Disabled.serialize(serializer),
            Reach::Any => Word::Any.serialize(serializer),
            Reach::Only(value) => value.serialize(serializer),
        }
    }
}

/// A word, or `T`: whichever the bytes are.
#[derive(Deserialize)]
#[serde(untagged)]
enum Repr<T> {
    Word(Word),
    Only(T),
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Reach<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(match Repr::deserialize(deserializer)? {
            Repr::Word(Word::Disabled) => Reach::Disabled,
            Repr::Word(Word::Any) => Reach::Any,
            Repr::Only(value) => Reach::Only(value),
        })
    }
}

