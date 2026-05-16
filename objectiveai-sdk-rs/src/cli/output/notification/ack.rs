use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Wire shape for silent-success notifications (`config set`,
/// `favorites add/del/edit`, `instructions clear`, etc.). Wire:
/// `{"type":"notification","ok":true}`.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[schemars(rename = "cli.output.notification.Ok")]
pub struct Ok {
    pub ok: bool,
}

pub const OK: Ok = Ok { ok: true };
