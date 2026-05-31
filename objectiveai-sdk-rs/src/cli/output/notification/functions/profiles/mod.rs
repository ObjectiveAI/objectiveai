mod pairs;

pub use pairs::*;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Result of `functions profiles get`.
///
/// Wire: `{"type":"notification","profile":{...GetProfileResponse...}}`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[schemars(rename = "cli.output.notification.functions.profiles.Profile")]
pub struct Profile {
    pub profile: crate::functions::profiles::response::GetProfileResponse,
}
