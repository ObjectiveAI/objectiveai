//! Whether a container holds a tagging tool, and if so how far each
//! side of it reaches.

use serde::de::{Deserializer, Error};
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};

use super::word::Word;

/// The reach of a tool that tags or untags, as a member of
/// [`DaemonTools`](super::DaemonTools) states it: not held, or held
/// with `T` saying which things and which tags, each side a
/// [`Within`](super::Within) of its own. There is no `any` at this
/// level, because "any agent with any tag" is `T` with both sides
/// `any`, and one word for it would hide which side is open. On the
/// wire the string `"disabled"`, or `T` itself, flat. `Disabled` is
/// the default, and what a member left out decodes as.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub enum Held<T> {
    /// The container does not hold the tool.
    #[default]
    Disabled,
    /// The container holds the tool, reaching what `T` says on each
    /// side.
    Only(T),
}

impl<T: Serialize> Serialize for Held<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Held::Disabled => Word::Disabled.serialize(serializer),
            Held::Only(value) => value.serialize(serializer),
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

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Held<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match Repr::deserialize(deserializer)? {
            Repr::Word(Word::Disabled) => Ok(Held::Disabled),
            Repr::Word(Word::Any) => Err(D::Error::custom(
                "a tagging tool is \"disabled\" or its two sides, never \"any\"",
            )),
            Repr::Only(value) => Ok(Held::Only(value)),
        }
    }
}
