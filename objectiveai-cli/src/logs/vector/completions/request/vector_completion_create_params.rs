//! `impl ProducesRequestFiles for VectorCompletionCreateParams`
//! (placeholder — monolithic summary write).

use objectiveai_sdk::vector::completions::request::VectorCompletionCreateParams;

use objectiveai_sdk::logs::LogReference;

use crate::filesystem::logs::LogFile;
use crate::logs::ProducesRequestFiles;

// Placeholder `ProducesRequestFiles` impl: dumps the whole params as
// one summary JSON without extracting any leaves. Lets the
// [`crate::filesystem::logs::LogWriter`]'s deferred-request pipeline
// stay homogeneous across factories while this type still uses the
// monolithic on-disk shape. Phase 2 will swap this for an actual
// per-leaf extraction (see `agent_completion_create_params.rs` for the
// reference pattern).
impl ProducesRequestFiles for VectorCompletionCreateParams {
    fn produce_files(
        &self,
        id: &str,
        route_base: &str,
    ) -> (LogReference, Vec<LogFile>) {
        let summary = LogFile {
            route: route_base.to_string(),
            id: id.to_string(),
            message_index: None,
            media_index: None,
            extension: "json".to_string(),
            content: serde_json::to_vec_pretty(self)
                .expect("VectorCompletionCreateParams serializes"),
        };
        let reference = LogReference::new(summary.path());
        (reference, vec![summary])
    }
}
