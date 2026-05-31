use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// `objectiveai --help` (and per-subcommand `--help`) output. The CLI
/// renders clap's help text into this struct so the help line is
/// part of the same JSONL stream as everything else.
///
/// Wire: `{"type":"notification","value":{"kind":"help","help":"…text…"}}`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[schemars(rename = "cli.output.notification.Help")]
pub struct Help {
    pub help: String,
}
