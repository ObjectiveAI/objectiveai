use rmcp::{
    ServerHandler,
    handler::server::router::tool::ToolRouter,
    handler::server::wrapper::Parameters,
    model::{
        CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities,
        ServerInfo,
    },
    schemars, tool, tool_handler, tool_router,
};

use crate::state::FileStateCache;

// --- Input schemas (matching Claude Code exactly) ---

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ReadRequest {
    #[schemars(description = "The absolute path to the file to read")]
    file_path: String,
    #[schemars(description = "The line number to start reading from. Only provide if the file is too large to read at once.")]
    offset: Option<usize>,
    #[schemars(description = "The number of lines to read. Only provide if the file is too large to read at once")]
    limit: Option<usize>,
    #[schemars(description = "Page range for PDF files (e.g., \"1-5\", \"3\", \"10-20\"). Only applicable to PDF files. Maximum 20 pages per request.")]
    pages: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct WriteRequest {
    #[schemars(description = "The absolute path to the file to write (must be absolute, not relative)")]
    file_path: String,
    #[schemars(description = "The content to write to the file")]
    content: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct EditRequest {
    #[schemars(description = "The absolute path to the file to modify")]
    file_path: String,
    #[schemars(description = "The text to replace")]
    old_string: String,
    #[schemars(description = "The text to replace it with (must be different from old_string)")]
    new_string: String,
    #[schemars(description = "Replace all occurrences of old_string (default false)")]
    replace_all: Option<bool>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct BashRequest {
    #[schemars(description = "The command to execute")]
    command: String,
    #[schemars(description = "Optional timeout in milliseconds (max 600000)")]
    timeout: Option<u64>,
    #[schemars(description = "Clear, concise description of what this command does in active voice")]
    description: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GlobRequest {
    #[schemars(description = "The glob pattern to match files against")]
    pattern: String,
    #[schemars(description = "The directory to search in. If not specified, the current working directory will be used.")]
    path: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GrepRequest {
    #[schemars(description = "The regular expression pattern to search for in file contents")]
    pattern: String,
    #[schemars(description = "File or directory to search in (rg PATH). Defaults to current working directory.")]
    path: Option<String>,
    #[schemars(description = "Glob pattern to filter files (e.g. \"*.js\", \"*.{ts,tsx}\")")]
    glob: Option<String>,
    #[schemars(description = "Output mode: \"content\" shows matching lines, \"files_with_matches\" shows only file paths (default), \"count\" shows match counts")]
    output_mode: Option<String>,
    #[serde(rename = "-B")]
    #[schemars(rename = "-B", description = "Number of lines to show before each match")]
    before: Option<usize>,
    #[serde(rename = "-A")]
    #[schemars(rename = "-A", description = "Number of lines to show after each match")]
    after: Option<usize>,
    #[serde(rename = "-C")]
    #[schemars(rename = "-C", description = "Alias for context.")]
    context_short: Option<usize>,
    #[schemars(description = "Number of lines to show before and after each match")]
    context: Option<usize>,
    #[serde(rename = "-n")]
    #[schemars(rename = "-n", description = "Show line numbers in output. Defaults to true.")]
    line_numbers: Option<bool>,
    #[serde(rename = "-i")]
    #[schemars(rename = "-i", description = "Case insensitive search")]
    case_insensitive: Option<bool>,
    #[serde(rename = "type")]
    #[schemars(rename = "type", description = "File type to search (e.g., \"js\", \"py\", \"rust\")")]
    file_type: Option<String>,
    #[schemars(description = "Limit output to first N lines/entries. Defaults to 250 when unspecified. Pass 0 for unlimited.")]
    head_limit: Option<usize>,
    #[schemars(description = "Skip first N lines/entries before applying head_limit. Defaults to 0.")]
    offset: Option<usize>,
    #[schemars(description = "Enable multiline mode where . matches newlines and patterns can span lines. Default: false.")]
    multiline: Option<bool>,
}

// --- Tool server ---

#[derive(Debug, Clone)]
pub struct FilesystemMcp {
    pub tool_router: ToolRouter<Self>,
    file_state: FileStateCache,
    shell_state: crate::bash::ShellState,
}

#[tool_router]
impl FilesystemMcp {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
            shell_state: crate::bash::ShellState::new(),
            file_state: FileStateCache::new(),
        }
    }

    /// Initialize session state (shell snapshot, etc.).
    /// Should be called once after construction.
    pub async fn init(&self) {
        self.shell_state.init_snapshot().await;
    }

    #[tool(name = "Read", description = "Reads a file from the local filesystem.")]
    async fn read(&self, Parameters(req): Parameters<ReadRequest>) -> Result<CallToolResult, rmcp::ErrorData> {
        match crate::read_file::read_file(&self.file_state, &req.file_path, req.offset, req.limit, req.pages.as_deref()).await {
            Ok(crate::read_file::ReadOutput::Text(json)) => {
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Ok(crate::read_file::ReadOutput::Image { base64, media_type }) => {
                Ok(CallToolResult::success(vec![Content::image(base64, media_type)]))
            }
            Ok(crate::read_file::ReadOutput::Notebook(blocks)) => {
                let contents: Vec<Content> = blocks.into_iter().map(|b| match b {
                    crate::notebook::NotebookBlock::Text(text) => Content::text(text),
                    crate::notebook::NotebookBlock::Image { base64, media_type } => {
                        Content::image(base64, media_type)
                    }
                }).collect();
                Ok(CallToolResult::success(contents))
            }
            Ok(crate::read_file::ReadOutput::FileUnchanged(stub)) => {
                Ok(CallToolResult::success(vec![Content::text(stub)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e)])),
        }
    }

    #[tool(name = "Write", description = "Writes a file to the local filesystem.")]
    async fn write(&self, Parameters(req): Parameters<WriteRequest>) -> String {
        match crate::write_file::write_file(&self.file_state, &req.file_path, &req.content).await {
            Ok(output) => output,
            Err(e) => e,
        }
    }

    #[tool(name = "Edit", description = "Performs exact string replacements in files.")]
    async fn edit(&self, Parameters(req): Parameters<EditRequest>) -> String {
        match crate::edit_file::edit_file(
            &self.file_state,
            &req.file_path,
            &req.old_string,
            &req.new_string,
            req.replace_all.unwrap_or(false),
        ).await {
            Ok(output) => output,
            Err(e) => e,
        }
    }

    #[tool(name = "Bash", description = "Executes a given bash command and returns its output.")]
    async fn bash(&self, Parameters(req): Parameters<BashRequest>) -> Content {
        match crate::bash::execute_bash(&self.shell_state, &req.command, req.timeout).await {
            Ok(output) => {
                if output.is_image {
                    if let Some(parsed) = crate::bash::parse_data_uri(&output.stdout) {
                        return Content::image(parsed.data, parsed.media_type);
                    }
                }
                let json = serde_json::to_string_pretty(&output).unwrap_or_default();
                Content::text(json)
            }
            Err(e) => Content::text(e),
        }
    }

    #[tool(name = "Glob", description = "Fast file pattern matching tool that works with any codebase size")]
    async fn glob(&self, Parameters(req): Parameters<GlobRequest>) -> String {
        match crate::glob_search::glob_search(&req.pattern, req.path.as_deref()).await {
            Ok(output) => {
                if output.contains("\"truncated\": true") {
                    format!("{output}\n(Results are truncated. Consider using a more specific path or pattern.)")
                } else {
                    output
                }
            }
            Err(e) => e,
        }
    }

    #[tool(name = "Grep", description = "A powerful search tool built on ripgrep")]
    async fn grep(&self, Parameters(req): Parameters<GrepRequest>) -> String {
        let input = crate::grep_search::GrepSearchInput {
            pattern: req.pattern,
            path: req.path,
            glob: req.glob,
            output_mode: req.output_mode,
            before: req.before,
            after: req.after,
            context_short: req.context_short,
            context: req.context,
            line_numbers: req.line_numbers,
            case_insensitive: req.case_insensitive,
            file_type: req.file_type,
            head_limit: req.head_limit,
            offset: req.offset,
            multiline: req.multiline,
        };
        match crate::grep_search::grep_search(&input).await {
            Ok(output) => output,
            Err(e) => e,
        }
    }
}

#[tool_handler]
impl ServerHandler for FilesystemMcp {
    fn get_info(&self) -> ServerInfo {
        // rmcp 1.7 marks `ServerInfo`/`Implementation` `#[non_exhaustive]`,
        // so build via `Default` + explicit field assignment.
        let mut server_info = Implementation::default();
        server_info.name = "oaifs".into();
        server_info.version = env!("CARGO_PKG_VERSION").into();
        let mut info = ServerInfo::default();
        info.protocol_version = ProtocolVersion::V_2025_06_18;
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info.server_info = server_info;
        info.instructions = None;
        info
    }
}
