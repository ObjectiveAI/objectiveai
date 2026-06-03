//! `impl ProducesRequestFiles for FunctionInventionRecursiveCreateParams`.

use objectiveai_sdk::functions::inventions::recursive::request::{
    FunctionInventionRecursiveCreateParams,
    FunctionInventionRecursiveCreateParamsLog,
};

use objectiveai_sdk::logs::LogReference;

use crate::filesystem::logs::LogFile;
use crate::logs::ProducesRequestFiles;

impl ProducesRequestFiles for FunctionInventionRecursiveCreateParams {
    /// Break this request out into per-leaf log files:
    /// - `state` → own JSON file under `<route_base>/state/`.
    /// - `continuation` (if Some) → own `.txt` file under
    ///   `<route_base>/continuation/`.
    /// - top-level Log envelope → `<route_base>/<id>.json`.
    fn produce_files(
        &self,
        id: &str,
        route_base: &str,
    ) -> (LogReference, Vec<LogFile>) {
        let mut all_files: Vec<LogFile> = Vec::new();

        // 1. state → own JSON file.
        let state_file = LogFile {
            route: format!("{route_base}/state"),
            id: id.to_string(),
            message_index: None,
            media_index: None,
            extension: "json".to_string(),
            content: serde_json::to_vec_pretty(&self.state)
                .expect("ParamsStateOrRemoteCommitOptional serializes"),
        };
        let state_ref = LogReference::new(state_file.path());
        all_files.push(state_file);

        // 2. continuation → own file.
        let continuation_ref = self.continuation.as_ref().map(|c| {
            let file = LogFile {
                route: format!("{route_base}/continuation"),
                id: id.to_string(),
                message_index: None,
                media_index: None,
                extension: "txt".to_string(),
                content: c.clone().into_bytes(),
            };
            let r = LogReference::new(file.path());
            all_files.push(file);
            r
        });

        // 3. Top-level envelope.
        let log = FunctionInventionRecursiveCreateParamsLog {
            remote: self.remote.clone(),
            overwrite: self.overwrite,
            state: state_ref,
            provider: self.provider.clone(),
            agent: self.agent.clone(),
            prompt: self.prompt.clone(),
            seed: self.seed,
            stream: self.stream,
            max_step_retries: self.max_step_retries,
            continuation: continuation_ref,
        };
        let summary_file = LogFile {
            route: route_base.to_string(),
            id: id.to_string(),
            message_index: None,
            media_index: None,
            extension: "json".to_string(),
            content: serde_json::to_vec_pretty(&log)
                .expect("FunctionInventionRecursiveCreateParamsLog serializes"),
        };
        let summary_ref = LogReference::new(summary_file.path());
        all_files.push(summary_file);

        (summary_ref, all_files)
    }
}
