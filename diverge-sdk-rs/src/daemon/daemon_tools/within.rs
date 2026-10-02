//! Any, or only what is named: one side of a tagging tool's reach.

use serde::de::{Deserializer, Error};
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};

use super::word::Word;

/// How far one side of a tagging tool reaches: every thing of the
/// caller's, or only what `T` names — a filter for which agents,
/// tools or templates, a list of tags for which tags. On the wire
/// the string `"any"`, or `T` itself, flat: an object for a filter,
/// an array for tags. There is no `disabled` here: the tool as a
/// whole is held or not, by [`Held`](super::Held), and each of its
/// sides is always one of these two.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Within<T> {
    /// Every one.
    Any,
    /// Only what `T` names. For tags, an empty list is no tag at
    /// all, and the tool acts with none.
    Only(T),
}

impl<T: Serialize> Serialize for Within<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Within::Any => Word::Any.serialize(serializer),
            Within::Only(value) => value.serialize(serializer),
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

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Within<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match Repr::deserialize(deserializer)? {
            Repr::Word(Word::Any) => Ok(Within::Any),
            Repr::Word(Word::Disabled) => Err(D::Error::custom(
                "a side of a tagging tool is \"any\" or what it names, never \"disabled\"",
            )),
            Repr::Only(value) => Ok(Within::Only(value)),
        }
    }
}
