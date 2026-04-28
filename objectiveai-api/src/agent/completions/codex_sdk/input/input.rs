use serde::{Deserialize, Serialize};

/// Mirrors Python's `Input = Union[str, List[Union[UserInput, Dict[str, Any]]]]`
/// (`types.py:80`). A turn input is either a plain string OR a list whose
/// elements may be typed [`super::UserInput`] entries or raw JSON objects
/// (Python's "accepts dicts for convenience" path).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Input {
    String(String),
    List(Vec<UserInputOrJson>),
}

/// One element of an [`Input::List`]. Mirrors Python's
/// `Union[UserInput, Dict[str, Any]]` — accept either a strongly-typed
/// `UserInput` or an arbitrary JSON object that the consumer hand-built.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum UserInputOrJson {
    Typed(super::UserInput),
    Raw(serde_json::Value),
}
