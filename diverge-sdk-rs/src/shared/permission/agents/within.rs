//! Every agent, the tool's attached agents, those never active, the
//! agents listed, or only these.

use std::borrow::Cow;

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::de::Deserializer;
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};

use super::Key;
use crate::shared::permission::Tags;

/// How far a grant over agents reaches: every agent there is, the
/// agents attached to the tool holding the grant, the agents that have
/// never been active, the agents listed, or only those the [`Tags`]
/// let through. Flat on the wire: the string `"any"`, the string
/// `"attached"`, the string `"never_active"`, the object
/// `{"agents":[…]}`, or the tags object as it is. No word is an
/// object, and neither object decodes as the other, so the five shapes
/// never collide; a string that is none of the words does not decode.
///
/// # `attached` is the agents attached to the tool
///
/// The one word the agents kind has that no other has, and that no
/// grant of the daemon's own has: it names nothing and reaches the
/// agents attached to the tool that holds the grant, judged at every
/// request, whether or not any of them is active. For a standalone
/// tool, those it is attached to directly, and every agent of every
/// template it is attached to. For a dependency tool, those attached
/// to it by a registration that declared its template, and still so:
/// for a transient tool the one agent it was deployed for. A tool that
/// may `message` within `attached` may message its own agents and no
/// other, whatever their tags, which is what a tool that answers to
/// its agents asks for.
///
/// # `never_active` is the agents that have never run
///
/// Every agent no container has ever been started for — no
/// `last_active` — judged per agent at every request, so an agent
/// leaves this reach the moment it first runs, and is reached after
/// only if another grant reaches it (`attached`, once it is
/// attached). What a tool asks for that must reach agents before
/// they could ever have declared it: to look one up, list them, or
/// send one its first message. The daemon judges it as its own grant
/// over agents whose filter is `{"active": "never"}`.
///
/// # `{"agents":[…]}` is exactly the agents listed
///
/// The one form that names anything, and it names agents only: each a
/// [`Key`], the template and index that are one agent's on the daemon
/// holding it. It reaches those agents and no other, whatever their
/// tags, their attachments or their activity, judged at every request;
/// one that does not exist, or no longer does, is reached by nothing.
/// What a tool asks for whose arguments are themselves a list of
/// agents — the program declares, at its registration, the agents its
/// arguments gave it, and is held to them. The list is one daemon's:
/// an index is counted by the daemon that made the agent, so a
/// declaration that lists agents is computed from the arguments at
/// every registration and is never a constant of the image. An empty
/// list reaches nothing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Within {
    /// Every agent: the string `"any"`.
    Any,
    /// The agents attached to the tool: the string `"attached"`.
    Attached,
    /// The agents that have never been active: the string
    /// `"never_active"`.
    NeverActive,
    /// Exactly the agents listed: the object `{"agents":[…]}`.
    Agents(Vec<Key>),
    /// Only the agents the tags let through, as a test.
    Only(Tags),
}

/// The listed form as it is on the wire: one member, `agents`, and no
/// other.
#[derive(Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct Listed {
    /// The agents reached.
    agents: Vec<Key>,
}

/// The listed form borrowed, to be written without a copy.
#[derive(Serialize)]
struct ListedRef<'a> {
    agents: &'a [Key],
}

impl Serialize for Within {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Within::Any => serializer.serialize_str("any"),
            Within::Attached => serializer.serialize_str("attached"),
            Within::NeverActive => serializer.serialize_str("never_active"),
            Within::Agents(agents) => ListedRef { agents }.serialize(serializer),
            Within::Only(only) => only.serialize(serializer),
        }
    }
}

/// The three words, so that each is read as a word and not as tags.
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Word {
    Any,
    Attached,
    NeverActive,
}

/// The three shapes, tried in order: a word, then the list, then the
/// tags. The list requires its one member and the tags refuse it, so
/// an object is one or the other and never both.
#[derive(Deserialize)]
#[serde(untagged)]
enum Repr {
    Word(Word),
    Listed(Listed),
    Only(Tags),
}

impl<'de> Deserialize<'de> for Within {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(match Repr::deserialize(deserializer)? {
            Repr::Word(Word::Any) => Within::Any,
            Repr::Word(Word::Attached) => Within::Attached,
            Repr::Word(Word::NeverActive) => Within::NeverActive,
            Repr::Listed(listed) => Within::Agents(listed.agents),
            Repr::Only(only) => Within::Only(only),
        })
    }
}

/// The five shapes as a schema: one of the words, the list, or the
/// tags' own. Written by hand because the derive follows a derived
/// serde and this type's serde is its own.
impl JsonSchema for Within {
    fn schema_name() -> Cow<'static, str> {
        "AgentsWithin".into()
    }

    fn schema_id() -> Cow<'static, str> {
        "diverge_sdk::shared::permission::agents::Within".into()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        let listed = generator.subschema_for::<Listed>();
        let only = generator.subschema_for::<Tags>();
        json_schema!({
            "anyOf": [
                { "type": "string", "const": "any" },
                { "type": "string", "const": "attached" },
                { "type": "string", "const": "never_active" },
                listed,
                only
            ]
        })
    }
}
