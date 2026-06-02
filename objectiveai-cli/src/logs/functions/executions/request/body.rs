//! `impl ProducesRequestFiles for FunctionExecutionCreateParams` —
//! port of the SDK request-side log-emission method.

use objectiveai_sdk::functions::executions::request::{
    FunctionExecutionCreateParams, FunctionExecutionCreateParamsLog,
};

use objectiveai_sdk::logs::LogReference;

use crate::filesystem::logs::LogFile;
use crate::logs::ProducesRequestFiles;

impl ProducesRequestFiles for FunctionExecutionCreateParams {
    /// Break this request out into per-leaf log files:
    /// - `input` → recursive tree of files under `<route_base>/input/`
    ///   (per `extract_to_files`).
    /// - `continuation` (if Some) → own `.txt` file under
    ///   `<route_base>/continuation/`.
    /// - top-level Log envelope → `<route_base>/<id>.json`.
    fn produce_files(
        &self,
        id: &str,
        route_base: &str,
    ) -> (LogReference, Vec<LogFile>) {
        let mut all_files: Vec<LogFile> = Vec::new();

        // 1. input → recursive extraction.
        let (input_log, input_files) =
            crate::logs::functions::expression::input_value::extract_to_files(
                self.input.clone(),
                route_base,
                id,
                "",
            );
        all_files.extend(input_files);

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
        let log = FunctionExecutionCreateParamsLog {
            function: self.function.clone(),
            profile: self.profile.clone(),
            retry_token: self.retry_token.clone(),
            from_cache: self.from_cache,
            reasoning: self.reasoning.clone(),
            strategy: self.strategy.clone(),
            input: input_log,
            split: self.split,
            invert: self.invert,
            provider: self.provider.clone(),
            seed: self.seed,
            stream: self.stream,
            continuation: continuation_ref,
        };
        let summary_file = LogFile {
            route: route_base.to_string(),
            id: id.to_string(),
            message_index: None,
            media_index: None,
            extension: "json".to_string(),
            content: serde_json::to_vec_pretty(&log)
                .expect("FunctionExecutionCreateParamsLog serializes"),
        };
        let summary_ref = LogReference::new(summary_file.path());
        all_files.push(summary_file);

        (summary_ref, all_files)
    }
}
