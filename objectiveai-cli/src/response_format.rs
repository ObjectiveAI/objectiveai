use clap::Args;

/// Shared response format arguments for agent completions.
#[derive(Args)]
pub struct ResponseFormatArgs {
    /// Use JSON object response format.
    #[arg(long, group = "response_format")]
    pub response_format_json_object: bool,

    /// Use JSON schema response format with inline JSON schema.
    #[arg(long, group = "response_format")]
    pub response_format_json_schema_inline: Option<String>,

    /// Use JSON schema response format with inline Python code that produces the schema.
    #[arg(long, group = "response_format")]
    pub response_format_json_schema_python_inline: Option<String>,

    /// Use JSON schema response format with a Python file that produces the schema.
    #[arg(long, group = "response_format")]
    pub response_format_json_schema_python_file: Option<std::path::PathBuf>,

    /// Use grammar response format.
    #[arg(long, group = "response_format")]
    pub response_format_grammar: Option<String>,

    /// Use Python response format.
    #[arg(long, group = "response_format")]
    pub response_format_python: bool,

    /// Use tool call response format — tool name.
    #[arg(long, group = "response_format", requires = "response_format_tool_call_description")]
    pub response_format_tool_call_name: Option<String>,

    /// Tool call response format — tool description.
    #[arg(long)]
    pub response_format_tool_call_description: Option<String>,

    /// Tool call response format — schema as inline JSON.
    #[arg(long, group = "tool_call_schema")]
    pub response_format_tool_call_schema_inline: Option<String>,

    /// Tool call response format — schema from inline Python code.
    #[arg(long, group = "tool_call_schema")]
    pub response_format_tool_call_schema_python_inline: Option<String>,

    /// Tool call response format — schema from a Python file.
    #[arg(long, group = "tool_call_schema")]
    pub response_format_tool_call_schema_python_file: Option<std::path::PathBuf>,

    /// Tool call response format — whether the tool call is required.
    #[arg(long)]
    pub response_format_tool_call_required: bool,
}

impl ResponseFormatArgs {
    pub fn resolve(self) -> Result<Option<objectiveai::agent::completions::request::ResponseFormat>, crate::error::Error> {
        use objectiveai::agent::completions::request::ResponseFormat;

        if self.response_format_json_object {
            return Ok(Some(ResponseFormat::JsonObject));
        }

        if let Some(inline) = self.response_format_json_schema_inline {
            let schema = {
                let mut de = serde_json::Deserializer::from_str(&inline);
                serde_path_to_error::deserialize(&mut de)
                    .map_err(crate::error::Error::InlineDeserialize)?
            };
            return Ok(Some(ResponseFormat::JsonSchema { schema }));
        }
        if let Some(code) = self.response_format_json_schema_python_inline {
            let schema = crate::python::exec_code(&code)?;
            return Ok(Some(ResponseFormat::JsonSchema { schema }));
        }
        if let Some(path) = self.response_format_json_schema_python_file {
            let schema = crate::python::exec_file(&path)?;
            return Ok(Some(ResponseFormat::JsonSchema { schema }));
        }

        if let Some(grammar) = self.response_format_grammar {
            return Ok(Some(ResponseFormat::Grammar { grammar }));
        }

        if self.response_format_python {
            return Ok(Some(ResponseFormat::Python));
        }

        if let Some(name) = self.response_format_tool_call_name {
            let description = self.response_format_tool_call_description.unwrap_or_default();
            let schema = if let Some(inline) = self.response_format_tool_call_schema_inline {
                let mut de = serde_json::Deserializer::from_str(&inline);
                serde_path_to_error::deserialize(&mut de)
                    .map_err(crate::error::Error::InlineDeserialize)?
            } else if let Some(code) = self.response_format_tool_call_schema_python_inline {
                crate::python::exec_code(&code)?
            } else if let Some(path) = self.response_format_tool_call_schema_python_file {
                crate::python::exec_file(&path)?
            } else {
                Default::default()
            };
            let required = if self.response_format_tool_call_required { Some(true) } else { None };
            return Ok(Some(ResponseFormat::ToolCall { name, description, schema, required }));
        }

        Ok(None)
    }
}
