use serde::{Deserialize, Serialize};

/// Untagged union over the leaf input types. Each leaf carries its own
/// `r#type` discriminator field, so serde dispatches by trying variants in
/// declaration order and matching whichever leaf's literal type accepts.
/// Mirrors Python's `UserInput = Union[TextInput, LocalImageInput]`
/// (`types.py:79`).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum UserInput {
    Text(super::TextInput),
    LocalImage(super::LocalImageInput),
}
