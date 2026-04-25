use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct CreateArgs {
    /// Docker image to use for the laboratory environment
    #[arg(long)]
    pub docker_image: String,

    /// Builder agent(s) — at least one required.
    /// Format: key=value,key=value (e.g. favorite=name or remote=github,owner=x,repository=y)
    #[arg(long, required = true)]
    pub builder_agent: Vec<crate::agent_ref::AgentRef>,

    /// Evaluation agent reference (e.g. favorite=name or remote=github,owner=x,repository=y)
    #[arg(long)]
    pub evaluation_agent: Option<crate::agent_ref::AgentRef>,

    #[command(flatten)]
    pub builder_messages: BuilderMessageSource,

    #[command(flatten)]
    pub evaluation_messages: EvaluationMessageSource,

    #[command(flatten)]
    pub evaluation_output_schema: EvaluationOutputSchemaSource,

    #[command(flatten)]
    pub builder_continuation: BuilderContinuationArgs,

    #[command(flatten)]
    pub evaluation_continuation: EvaluationContinuationArgs,

    /// Output python script
    #[command(flatten)]
    pub output_python: OutputPythonSource,

    /// Maximum number of evaluation retries if validation fails.
    #[arg(long)]
    pub max_evaluation_retries: Option<u32>,

    /// Seed for deterministic mock responses
    #[arg(long)]
    pub seed: Option<i64>,

    #[command(flatten)]
    pub instructions: crate::instructions::InstructionsIdArg,
}

/// Output python script source — file or inline.
#[derive(Args)]
#[group(multiple = false)]
pub struct OutputPythonSource {
    /// Inline Python output scoring code
    #[arg(long)]
    pub output_python_inline: Option<String>,

    /// Path to a Python output scoring file
    #[arg(long)]
    pub output_python_file: Option<PathBuf>,
}

/// Messages for builder agents.
#[derive(Args)]
#[group(required = true, multiple = false)]
pub struct BuilderMessageSource {
    #[arg(long)]
    pub builder_messages_inline: Option<String>,
    #[arg(long)]
    pub builder_messages_python_inline: Option<String>,
    #[arg(long)]
    pub builder_messages_python_file: Option<PathBuf>,
}

impl BuilderMessageSource {
    pub fn resolve(self) -> Result<Vec<objectiveai::agent::completions::message::Message>, crate::error::Error> {
        if let Some(inline) = self.builder_messages_inline {
            let mut de = serde_json::Deserializer::from_str(&inline);
            return serde_path_to_error::deserialize(&mut de)
                .map_err(crate::error::Error::InlineDeserialize);
        }
        if let Some(code) = self.builder_messages_python_inline {
            return crate::python::exec_code(&code);
        }
        if let Some(path) = self.builder_messages_python_file {
            return crate::python::exec_file(&path);
        }
        unreachable!("clap group ensures one is set")
    }
}

/// Messages for the evaluation agent.
#[derive(Args)]
#[group(multiple = false)]
pub struct EvaluationMessageSource {
    #[arg(long)]
    pub evaluation_messages_inline: Option<String>,
    #[arg(long)]
    pub evaluation_messages_python_inline: Option<String>,
    #[arg(long)]
    pub evaluation_messages_python_file: Option<PathBuf>,
}

impl EvaluationMessageSource {
    pub fn resolve(self) -> Result<Option<Vec<objectiveai::agent::completions::message::Message>>, crate::error::Error> {
        if let Some(inline) = self.evaluation_messages_inline {
            let mut de = serde_json::Deserializer::from_str(&inline);
            return serde_path_to_error::deserialize(&mut de)
                .map(Some)
                .map_err(crate::error::Error::InlineDeserialize);
        }
        if let Some(code) = self.evaluation_messages_python_inline {
            return crate::python::exec_code(&code).map(Some);
        }
        if let Some(path) = self.evaluation_messages_python_file {
            return crate::python::exec_file(&path).map(Some);
        }
        Ok(None)
    }
}

/// Evaluation output schema source (objectiveai-rs InputSchema as JSON).
#[derive(Args)]
#[group(multiple = false)]
pub struct EvaluationOutputSchemaSource {
    #[arg(long)]
    pub evaluation_output_schema_inline: Option<String>,
    #[arg(long)]
    pub evaluation_output_schema_python_inline: Option<String>,
    #[arg(long)]
    pub evaluation_output_schema_python_file: Option<PathBuf>,
}

impl EvaluationOutputSchemaSource {
    pub fn resolve(self) -> Result<Option<objectiveai::functions::expression::InputSchema>, crate::error::Error> {
        if let Some(inline) = self.evaluation_output_schema_inline {
            let mut de = serde_json::Deserializer::from_str(&inline);
            return serde_path_to_error::deserialize(&mut de)
                .map(Some)
                .map_err(crate::error::Error::InlineDeserialize);
        }
        if let Some(code) = self.evaluation_output_schema_python_inline {
            return crate::python::exec_code(&code).map(Some);
        }
        if let Some(path) = self.evaluation_output_schema_python_file {
            return crate::python::exec_file(&path).map(Some);
        }
        Ok(None)
    }
}

/// Continuation for builder agents.
#[derive(Args)]
pub struct BuilderContinuationArgs {
    #[arg(long, group = "builder_continuation")]
    pub builder_openrouter_continuation_from_response: Option<String>,
    #[arg(long, group = "builder_continuation")]
    pub builder_claude_agent_sdk_continuation_from_response: Option<String>,
    #[arg(long, group = "builder_continuation")]
    pub builder_mock_continuation_from_response: Option<String>,
    #[arg(long, group = "builder_continuation")]
    pub builder_openrouter_continuation_messages_inline: Option<String>,
    #[arg(long, group = "builder_continuation")]
    pub builder_openrouter_continuation_messages_python_inline: Option<String>,
    #[arg(long, group = "builder_continuation")]
    pub builder_openrouter_continuation_messages_python_file: Option<PathBuf>,
    #[arg(long, group = "builder_continuation")]
    pub builder_mock_continuation_messages_inline: Option<String>,
    #[arg(long, group = "builder_continuation")]
    pub builder_mock_continuation_messages_python_inline: Option<String>,
    #[arg(long, group = "builder_continuation")]
    pub builder_mock_continuation_messages_python_file: Option<PathBuf>,
    #[arg(long, group = "builder_continuation")]
    pub builder_claude_agent_sdk_continuation_session_id: Option<String>,
}

impl BuilderContinuationArgs {
    pub fn resolve(self) -> Result<Option<String>, crate::error::Error> {
        crate::continuation::resolve_continuation(crate::continuation::ContinuationFields {
            openrouter_from_response: self.builder_openrouter_continuation_from_response,
            claude_agent_sdk_from_response: self.builder_claude_agent_sdk_continuation_from_response,
            mock_from_response: self.builder_mock_continuation_from_response,
            openrouter_messages_inline: self.builder_openrouter_continuation_messages_inline,
            openrouter_messages_python_inline: self.builder_openrouter_continuation_messages_python_inline,
            openrouter_messages_python_file: self.builder_openrouter_continuation_messages_python_file,
            mock_messages_inline: self.builder_mock_continuation_messages_inline,
            mock_messages_python_inline: self.builder_mock_continuation_messages_python_inline,
            mock_messages_python_file: self.builder_mock_continuation_messages_python_file,
            claude_agent_sdk_session_id: self.builder_claude_agent_sdk_continuation_session_id,
        })
    }
}

/// Continuation for the evaluation agent.
#[derive(Args)]
pub struct EvaluationContinuationArgs {
    #[arg(long, group = "evaluation_continuation")]
    pub evaluation_openrouter_continuation_from_response: Option<String>,
    #[arg(long, group = "evaluation_continuation")]
    pub evaluation_claude_agent_sdk_continuation_from_response: Option<String>,
    #[arg(long, group = "evaluation_continuation")]
    pub evaluation_mock_continuation_from_response: Option<String>,
    #[arg(long, group = "evaluation_continuation")]
    pub evaluation_openrouter_continuation_messages_inline: Option<String>,
    #[arg(long, group = "evaluation_continuation")]
    pub evaluation_openrouter_continuation_messages_python_inline: Option<String>,
    #[arg(long, group = "evaluation_continuation")]
    pub evaluation_openrouter_continuation_messages_python_file: Option<PathBuf>,
    #[arg(long, group = "evaluation_continuation")]
    pub evaluation_mock_continuation_messages_inline: Option<String>,
    #[arg(long, group = "evaluation_continuation")]
    pub evaluation_mock_continuation_messages_python_inline: Option<String>,
    #[arg(long, group = "evaluation_continuation")]
    pub evaluation_mock_continuation_messages_python_file: Option<PathBuf>,
    #[arg(long, group = "evaluation_continuation")]
    pub evaluation_claude_agent_sdk_continuation_session_id: Option<String>,
}

impl EvaluationContinuationArgs {
    pub fn resolve(self) -> Result<Option<String>, crate::error::Error> {
        crate::continuation::resolve_continuation(crate::continuation::ContinuationFields {
            openrouter_from_response: self.evaluation_openrouter_continuation_from_response,
            claude_agent_sdk_from_response: self.evaluation_claude_agent_sdk_continuation_from_response,
            mock_from_response: self.evaluation_mock_continuation_from_response,
            openrouter_messages_inline: self.evaluation_openrouter_continuation_messages_inline,
            openrouter_messages_python_inline: self.evaluation_openrouter_continuation_messages_python_inline,
            openrouter_messages_python_file: self.evaluation_openrouter_continuation_messages_python_file,
            mock_messages_inline: self.evaluation_mock_continuation_messages_inline,
            mock_messages_python_inline: self.evaluation_mock_continuation_messages_python_inline,
            mock_messages_python_file: self.evaluation_mock_continuation_messages_python_file,
            claude_agent_sdk_session_id: self.evaluation_claude_agent_sdk_continuation_session_id,
        })
    }
}
