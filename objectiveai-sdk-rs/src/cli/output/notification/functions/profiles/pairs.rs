use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Result of `functions profiles pairs get`. The CLI fetches both
/// halves of the pair and emits them together.
///
/// Wire: `{"type":"notification","pair":{"function":...,"profile":...}}`.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[schemars(rename = "cli.output.notification.functions.profiles.Pair")]
pub struct Pair {
    pub pair: FunctionProfilePair,
}

/// The composite body inside a `Pair` notification.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[schemars(rename = "cli.output.notification.functions.profiles.FunctionProfilePair")]
pub struct FunctionProfilePair {
    pub function: crate::functions::response::GetFunctionResponse,
    pub profile: crate::functions::profiles::response::GetProfileResponse,
}
