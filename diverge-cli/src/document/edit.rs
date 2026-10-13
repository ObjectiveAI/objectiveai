//! One place in a document, set or deleted: the program an `edit` is.

use serde_json::Value;

use super::Error;

/// The jq program an `edit` runs: `(SEL) = $value`, the place
/// `selector` names assigned the value bound as `$value`; or, with
/// `delete`, `del(SEL)`, the place removed. The selector is placed in
/// the program as written, and the value never is: it travels as a
/// variable, so nothing in it is read as jq.
pub fn program(selector: &str, delete: bool) -> String {
    if delete {
        format!("del({selector})")
    } else {
        format!("({selector}) = $value")
    }
}

/// The value an `edit` sets, read from `text` as YAML — a number, a
/// bool, a string, a list, an object, as YAML reads it, JSON being
/// YAML — as JSON for the program. Text that is not YAML is
/// [`Error::Value`].
pub fn value(text: &str) -> Result<Value, Error> {
    serde_path_to_error::deserialize(serde_yaml_ng::Deserializer::from_str(text)).map_err(Error::Value)
}
