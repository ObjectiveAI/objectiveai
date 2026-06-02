/// Error returned by `TryFrom<Args> for Request` for leaves that
/// deserialize one or more JSON-blob `Args` fields into typed `Request`
/// fields.
///
/// `field` names the Args field whose inline-JSON failed to parse
/// (e.g. `"body_inline"`, `"function_inline"`, `"dangerous_advanced"`);
/// `source` carries the `serde_path_to_error` wrapper so the JSON
/// location of the failure is preserved.
#[derive(Debug)]
pub struct FromArgsError {
    pub field: &'static str,
    pub source: serde_path_to_error::Error<serde_json::Error>,
}
