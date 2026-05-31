use crate::{agent, functions};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// Parameters for creating a function execution.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.executions.request.FunctionExecutionCreateParams")]
pub struct FunctionExecutionCreateParams {
    /// The function to execute (inline definition or remote path).
    pub function: functions::FullInlineFunctionOrRemoteCommitOptional,
    /// The profile to use (inline definition or remote path).
    pub profile: functions::InlineProfileOrRemoteCommitOptional,

    // --- Caching and retry options ---
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub retry_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub from_cache: Option<bool>,

    // --- Reasoning configuration ---
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub reasoning: Option<super::Reasoning>,

    // --- Core configuration ---
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub strategy: Option<super::Strategy>,
    pub input: functions::expression::InputValue,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub split: Option<bool>,
    /// If `true`, invert every output in the streamed response *after* the
    /// inner function has finished computing — scalar outputs become
    /// `1 - x`, vector outputs are reversed in place. The expression
    /// evaluator inside the function still sees the original scores; only
    /// the chunks delivered to the client (and the aggregated response
    /// passed to the usage handler) are inverted. Useful when a function
    /// is naturally written to score "lower is better" but the consumer
    /// wants "higher is better", or vice versa.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub invert: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub provider: Option<agent::completions::request::Provider>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub seed: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub stream: Option<bool>,
    /// Continuation from a previous completion, as a base64-encoded string.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub continuation: Option<String>,
}

#[cfg(feature = "filesystem")]
impl crate::filesystem::logs::ProducesRequestFiles for FunctionExecutionCreateParams {
    /// Break this request out into per-leaf log files:
    /// - `input` → recursive tree of files under `<route_base>/input/`
    ///   (per [`crate::functions::expression::InputValue::extract_to_files`]).
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

        // 1. input → recursive extraction.
        let (input_log, input_files) =
            self.input.clone().extract_to_files(route_base, id, "");
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
        let log = super::FunctionExecutionCreateParamsLog {
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
