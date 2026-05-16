use clap::Args;

/// Resolved continuation fields, extracted from any prefixed arg struct.
pub struct ContinuationFields {
    pub openrouter_from_response: Option<String>,
    pub claude_agent_sdk_from_response: Option<String>,
    pub mock_from_response: Option<String>,
    pub openrouter_messages_inline: Option<String>,
    pub openrouter_messages_python_inline: Option<String>,
    pub openrouter_messages_python_file: Option<std::path::PathBuf>,
    pub mock_messages_inline: Option<String>,
    pub mock_messages_python_inline: Option<String>,
    pub mock_messages_python_file: Option<std::path::PathBuf>,
    pub claude_agent_sdk_session_id: Option<String>,
}

/// Resolves continuation fields into a base64-encoded continuation string,
/// or None if no continuation was provided.
pub fn resolve_continuation(fields: ContinuationFields) -> Result<Option<String>, crate::error::Error> {
    // From response — already base64-encoded, pass through.
    if let Some(s) = fields.openrouter_from_response {
        return Ok(Some(s));
    }
    if let Some(s) = fields.claude_agent_sdk_from_response {
        return Ok(Some(s));
    }
    if let Some(s) = fields.mock_from_response {
        return Ok(Some(s));
    }

    // OpenRouter messages
    if let Some(inline) = fields.openrouter_messages_inline {
        let messages: Vec<objectiveai_sdk::agent::completions::message::Message> = {
            let mut de = serde_json::Deserializer::from_str(&inline);
            serde_path_to_error::deserialize(&mut de)
                .map_err(crate::error::Error::InlineDeserialize)?
        };
        let cont = objectiveai_sdk::agent::Continuation::Openrouter(
            objectiveai_sdk::agent::openrouter::Continuation {
                upstream: objectiveai_sdk::agent::openrouter::Upstream::Openrouter,
                messages,
                mcp_sessions: Default::default(),
            },
        );
        return Ok(Some(cont.to_string()));
    }
    if let Some(code) = fields.openrouter_messages_python_inline {
        let messages: Vec<objectiveai_sdk::agent::completions::message::Message> =
            crate::python::exec_code(&code)?;
        let cont = objectiveai_sdk::agent::Continuation::Openrouter(
            objectiveai_sdk::agent::openrouter::Continuation {
                upstream: objectiveai_sdk::agent::openrouter::Upstream::Openrouter,
                messages,
                mcp_sessions: Default::default(),
            },
        );
        return Ok(Some(cont.to_string()));
    }
    if let Some(path) = fields.openrouter_messages_python_file {
        let messages: Vec<objectiveai_sdk::agent::completions::message::Message> =
            crate::python::exec_file(&path)?;
        let cont = objectiveai_sdk::agent::Continuation::Openrouter(
            objectiveai_sdk::agent::openrouter::Continuation {
                upstream: objectiveai_sdk::agent::openrouter::Upstream::Openrouter,
                messages,
                mcp_sessions: Default::default(),
            },
        );
        return Ok(Some(cont.to_string()));
    }

    // Mock messages
    if let Some(inline) = fields.mock_messages_inline {
        let messages: Vec<objectiveai_sdk::agent::completions::message::Message> = {
            let mut de = serde_json::Deserializer::from_str(&inline);
            serde_path_to_error::deserialize(&mut de)
                .map_err(crate::error::Error::InlineDeserialize)?
        };
        let cont = objectiveai_sdk::agent::Continuation::Mock(
            objectiveai_sdk::agent::mock::Continuation {
                upstream: objectiveai_sdk::agent::mock::Upstream::Mock,
                messages,
                mcp_sessions: Default::default(),
            },
        );
        return Ok(Some(cont.to_string()));
    }
    if let Some(code) = fields.mock_messages_python_inline {
        let messages: Vec<objectiveai_sdk::agent::completions::message::Message> =
            crate::python::exec_code(&code)?;
        let cont = objectiveai_sdk::agent::Continuation::Mock(
            objectiveai_sdk::agent::mock::Continuation {
                upstream: objectiveai_sdk::agent::mock::Upstream::Mock,
                messages,
                mcp_sessions: Default::default(),
            },
        );
        return Ok(Some(cont.to_string()));
    }
    if let Some(path) = fields.mock_messages_python_file {
        let messages: Vec<objectiveai_sdk::agent::completions::message::Message> =
            crate::python::exec_file(&path)?;
        let cont = objectiveai_sdk::agent::Continuation::Mock(
            objectiveai_sdk::agent::mock::Continuation {
                upstream: objectiveai_sdk::agent::mock::Upstream::Mock,
                messages,
                mcp_sessions: Default::default(),
            },
        );
        return Ok(Some(cont.to_string()));
    }

    // Claude Agent SDK session ID
    if let Some(session_id) = fields.claude_agent_sdk_session_id {
        let cont = objectiveai_sdk::agent::Continuation::ClaudeAgentSdk(
            objectiveai_sdk::agent::claude_agent_sdk::Continuation {
                upstream: objectiveai_sdk::agent::claude_agent_sdk::Upstream::ClaudeAgentSdk,
                session_id,
                mcp_sessions: Default::default(),
            },
        );
        return Ok(Some(cont.to_string()));
    }

    Ok(None)
}

/// Shared continuation arguments for commands that support multi-turn conversations.
#[derive(Args)]
pub struct ContinuationArgs {
    /// OpenRouter continuation from a previous response (base64-encoded).
    #[arg(long, group = "continuation")]
    pub openrouter_continuation_from_response: Option<String>,

    /// Claude Agent SDK continuation from a previous response (base64-encoded).
    #[arg(long, group = "continuation")]
    pub claude_agent_sdk_continuation_from_response: Option<String>,

    /// Mock continuation from a previous response (base64-encoded).
    #[arg(long, group = "continuation")]
    pub mock_continuation_from_response: Option<String>,

    /// OpenRouter continuation messages as inline JSON.
    #[arg(long, group = "continuation")]
    pub openrouter_continuation_messages_inline: Option<String>,

    /// OpenRouter continuation messages from inline Python code.
    #[arg(long, group = "continuation")]
    pub openrouter_continuation_messages_python_inline: Option<String>,

    /// OpenRouter continuation messages from a Python file.
    #[arg(long, group = "continuation")]
    pub openrouter_continuation_messages_python_file: Option<std::path::PathBuf>,

    /// Mock continuation messages as inline JSON.
    #[arg(long, group = "continuation")]
    pub mock_continuation_messages_inline: Option<String>,

    /// Mock continuation messages from inline Python code.
    #[arg(long, group = "continuation")]
    pub mock_continuation_messages_python_inline: Option<String>,

    /// Mock continuation messages from a Python file.
    #[arg(long, group = "continuation")]
    pub mock_continuation_messages_python_file: Option<std::path::PathBuf>,

    /// Claude Agent SDK continuation with a session ID.
    #[arg(long, group = "continuation")]
    pub claude_agent_sdk_continuation_session_id: Option<String>,
}

impl ContinuationArgs {
    pub fn resolve(self) -> Result<Option<String>, crate::error::Error> {
        resolve_continuation(ContinuationFields {
            openrouter_from_response: self.openrouter_continuation_from_response,
            claude_agent_sdk_from_response: self.claude_agent_sdk_continuation_from_response,
            mock_from_response: self.mock_continuation_from_response,
            openrouter_messages_inline: self.openrouter_continuation_messages_inline,
            openrouter_messages_python_inline: self.openrouter_continuation_messages_python_inline,
            openrouter_messages_python_file: self.openrouter_continuation_messages_python_file,
            mock_messages_inline: self.mock_continuation_messages_inline,
            mock_messages_python_inline: self.mock_continuation_messages_python_inline,
            mock_messages_python_file: self.mock_continuation_messages_python_file,
            claude_agent_sdk_session_id: self.claude_agent_sdk_continuation_session_id,
        })
    }
}
