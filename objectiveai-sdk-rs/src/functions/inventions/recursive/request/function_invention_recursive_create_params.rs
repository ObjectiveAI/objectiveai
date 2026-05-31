use crate::{agent, functions};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.inventions.recursive.request.FunctionInventionRecursiveCreateParams")]
pub struct FunctionInventionRecursiveCreateParams {
    pub remote: crate::Remote,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub overwrite: Option<bool>,
    pub state: functions::inventions::ParamsStateOrRemoteCommitOptional,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub provider: Option<agent::completions::request::Provider>,
    pub agent: agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional,
    pub prompt: functions::inventions::prompts::InlinePromptOrRemoteCommitOptional,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub seed: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub stream: Option<bool>,
    /// Maximum number of retries per invention step.
    /// Each step is one agent completion (which itself may loop internally
    /// via tool calls). If the step's validation still fails after the
    /// agent loop ends, the step is retried up to this many times.
    /// Defaults to 3 if not specified.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub max_step_retries: Option<u32>,
    /// Continuation from a previous completion, as a base64-encoded string.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub continuation: Option<String>,
}

#[cfg(feature = "filesystem")]
impl crate::filesystem::logs::ProducesRequestFiles for FunctionInventionRecursiveCreateParams {
    /// Break this request out into per-leaf log files:
    /// - `state` → own JSON file under `<route_base>/state/`.
    /// - `continuation` (if Some) → own `.txt` file under
    ///   `<route_base>/continuation/`.
    /// - top-level Log envelope → `<route_base>/<id>.json`.
    fn produce_files(
        &self,
        id: &str,
        route_base: &str,
    ) -> (
        crate::filesystem::logs::LogReference,
        Vec<crate::filesystem::logs::LogFile>,
    ) {
        use crate::filesystem::logs::{LogFile, LogReference};

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
        let log = super::FunctionInventionRecursiveCreateParamsLog {
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
