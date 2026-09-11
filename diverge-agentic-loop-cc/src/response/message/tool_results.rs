//! What the server-side tools answer with.
//!
//! Each server tool's result block carries a content union of its
//! own — a result shape or an error shell — and each arm announces
//! itself with a `type` literal, so the untagged unions discriminate
//! the usual way. Ported from the api crate's newer-SDK port, with
//! this crate's doctrine applied: error codes and tool names are
//! API-owned open vocabularies and ride as [`String`]s, and counters
//! that can be absent default rather than fail.

use serde::Deserialize;

use super::DocumentBlock;

// ---------------------------------------------------------------
// Web search
// ---------------------------------------------------------------

/// A web search's content: results, or the error shell.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum WebSearchToolResultContent {
    /// The search failed.
    Error(WebSearchToolResultError),
    /// The results, one block per hit.
    Results(Vec<WebSearchResultBlock>),
}

/// A web search's error shell.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct WebSearchToolResultError {
    /// Always `web_search_tool_result_error`.
    pub r#type: WebSearchToolResultErrorType,
    /// Why it failed — an API-owned vocabulary, open.
    pub error_code: String,
}

/// The `web_search_tool_result_error` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum WebSearchToolResultErrorType {
    /// The only value.
    #[default]
    WebSearchToolResultError,
}

/// One web search hit.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct WebSearchResultBlock {
    /// Always `web_search_result`.
    pub r#type: WebSearchResultType,
    /// The hit's content, encrypted for citation use.
    pub encrypted_content: String,
    /// How old the page is, when known.
    pub page_age: Option<String>,
    /// The page's title.
    pub title: String,
    /// The page's URL.
    pub url: String,
}

/// The `web_search_result` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum WebSearchResultType {
    /// The only value.
    #[default]
    WebSearchResult,
}

// ---------------------------------------------------------------
// Web fetch
// ---------------------------------------------------------------

/// A web fetch's content: the fetched document, or the error shell.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum WebFetchToolResultContent {
    /// The fetch failed.
    Error(WebFetchToolResultError),
    /// The fetched page.
    Result(WebFetchBlock),
}

/// A web fetch's error shell.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct WebFetchToolResultError {
    /// Always `web_fetch_tool_result_error`.
    pub r#type: WebFetchToolResultErrorType,
    /// Why it failed — an API-owned vocabulary, open.
    pub error_code: String,
}

/// The `web_fetch_tool_result_error` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum WebFetchToolResultErrorType {
    /// The only value.
    #[default]
    WebFetchToolResultError,
}

/// The fetched page: a document, and where it came from.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct WebFetchBlock {
    /// Always `web_fetch_result`.
    pub r#type: WebFetchResultType,
    /// The page, as a document.
    pub content: DocumentBlock,
    /// When it was fetched, when known.
    pub retrieved_at: Option<String>,
    /// The URL fetched.
    pub url: String,
}

/// The `web_fetch_result` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum WebFetchResultType {
    /// The only value.
    #[default]
    WebFetchResult,
}

// ---------------------------------------------------------------
// Code execution
// ---------------------------------------------------------------

/// A code execution's content: a result, its encrypted twin, or the
/// error shell.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum CodeExecutionToolResultContent {
    /// The execution failed to run at all.
    Error(CodeExecutionToolResultError),
    /// It ran, stdout in the clear.
    Result(CodeExecutionResultBlock),
    /// It ran, stdout encrypted.
    EncryptedResult(EncryptedCodeExecutionResultBlock),
}

/// A code execution's error shell.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct CodeExecutionToolResultError {
    /// Always `code_execution_tool_result_error`.
    pub r#type: CodeExecutionToolResultErrorType,
    /// Why it failed — an API-owned vocabulary, open.
    pub error_code: String,
}

/// The `code_execution_tool_result_error` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum CodeExecutionToolResultErrorType {
    /// The only value.
    #[default]
    CodeExecutionToolResultError,
}

/// What a code execution produced.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct CodeExecutionResultBlock {
    /// Always `code_execution_result`.
    pub r#type: CodeExecutionResultType,
    /// Files it produced.
    pub content: Vec<CodeExecutionOutputBlock>,
    /// Its exit code — genuinely signed.
    pub return_code: i64,
    /// What it wrote to stderr.
    pub stderr: String,
    /// What it wrote to stdout.
    pub stdout: String,
}

/// The `code_execution_result` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum CodeExecutionResultType {
    /// The only value.
    #[default]
    CodeExecutionResult,
}

/// What a code execution produced, stdout withheld.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct EncryptedCodeExecutionResultBlock {
    /// Always `encrypted_code_execution_result`.
    pub r#type: EncryptedCodeExecutionResultType,
    /// Files it produced.
    pub content: Vec<CodeExecutionOutputBlock>,
    /// Stdout, encrypted.
    pub encrypted_stdout: String,
    /// Its exit code — genuinely signed.
    pub return_code: i64,
    /// What it wrote to stderr.
    pub stderr: String,
}

/// The `encrypted_code_execution_result` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum EncryptedCodeExecutionResultType {
    /// The only value.
    #[default]
    EncryptedCodeExecutionResult,
}

/// One file a code execution produced.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct CodeExecutionOutputBlock {
    /// Always `code_execution_output`.
    pub r#type: CodeExecutionOutputType,
    /// The file, by id.
    pub file_id: String,
}

/// The `code_execution_output` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum CodeExecutionOutputType {
    /// The only value.
    #[default]
    CodeExecutionOutput,
}

// ---------------------------------------------------------------
// Bash code execution
// ---------------------------------------------------------------

/// A bash execution's content: a result, or the error shell.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum BashCodeExecutionToolResultContent {
    /// The execution failed to run at all.
    Error(BashCodeExecutionToolResultError),
    /// It ran.
    Result(BashCodeExecutionResultBlock),
}

/// A bash execution's error shell.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct BashCodeExecutionToolResultError {
    /// Always `bash_code_execution_tool_result_error`.
    pub r#type: BashCodeExecutionToolResultErrorType,
    /// Why it failed — an API-owned vocabulary, open.
    pub error_code: String,
}

/// The `bash_code_execution_tool_result_error` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum BashCodeExecutionToolResultErrorType {
    /// The only value.
    #[default]
    BashCodeExecutionToolResultError,
}

/// What a bash execution produced.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct BashCodeExecutionResultBlock {
    /// Always `bash_code_execution_result`.
    pub r#type: BashCodeExecutionResultType,
    /// Files it produced.
    pub content: Vec<BashCodeExecutionOutputBlock>,
    /// Its exit code — genuinely signed.
    pub return_code: i64,
    /// What it wrote to stderr.
    pub stderr: String,
    /// What it wrote to stdout.
    pub stdout: String,
}

/// The `bash_code_execution_result` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum BashCodeExecutionResultType {
    /// The only value.
    #[default]
    BashCodeExecutionResult,
}

/// One file a bash execution produced.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct BashCodeExecutionOutputBlock {
    /// Always `bash_code_execution_output`.
    pub r#type: BashCodeExecutionOutputType,
    /// The file, by id.
    pub file_id: String,
}

/// The `bash_code_execution_output` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum BashCodeExecutionOutputType {
    /// The only value.
    #[default]
    BashCodeExecutionOutput,
}

// ---------------------------------------------------------------
// Text editor code execution
// ---------------------------------------------------------------

/// A text-editor execution's content: one of three result shapes,
/// or the error shell.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum TextEditorCodeExecutionToolResultContent {
    /// The operation failed.
    Error(TextEditorCodeExecutionToolResultError),
    /// A view of a file.
    ViewResult(TextEditorCodeExecutionViewResultBlock),
    /// A file created or replaced.
    CreateResult(TextEditorCodeExecutionCreateResultBlock),
    /// A string replacement made.
    StrReplaceResult(TextEditorCodeExecutionStrReplaceResultBlock),
}

/// A text-editor execution's error shell — the one with a message.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct TextEditorCodeExecutionToolResultError {
    /// Always `text_editor_code_execution_tool_result_error`.
    pub r#type: TextEditorCodeExecutionToolResultErrorType,
    /// Why it failed — an API-owned vocabulary, open.
    pub error_code: String,
    /// The failure in words, when the API says more.
    pub error_message: Option<String>,
}

/// The `text_editor_code_execution_tool_result_error` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum TextEditorCodeExecutionToolResultErrorType {
    /// The only value.
    #[default]
    TextEditorCodeExecutionToolResultError,
}

/// A view of a file.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TextEditorCodeExecutionViewResultBlock {
    /// Always `text_editor_code_execution_view_result`.
    pub r#type: TextEditorCodeExecutionViewResultType,
    /// What was seen.
    pub content: String,
    /// What kind of file it is — an API-owned vocabulary, open.
    pub file_type: String,
    /// How many lines were viewed.
    pub num_lines: Option<u64>,
    /// Where the view started.
    pub start_line: Option<u64>,
    /// How many lines the file has.
    pub total_lines: Option<u64>,
}

/// The `text_editor_code_execution_view_result` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum TextEditorCodeExecutionViewResultType {
    /// The only value.
    #[default]
    TextEditorCodeExecutionViewResult,
}

/// A file created — or replaced, which the flag says.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct TextEditorCodeExecutionCreateResultBlock {
    /// Always `text_editor_code_execution_create_result`.
    pub r#type: TextEditorCodeExecutionCreateResultType,
    /// Whether an existing file was updated rather than created.
    pub is_file_update: bool,
}

/// The `text_editor_code_execution_create_result` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum TextEditorCodeExecutionCreateResultType {
    /// The only value.
    #[default]
    TextEditorCodeExecutionCreateResult,
}

/// A string replacement, and where it landed.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TextEditorCodeExecutionStrReplaceResultBlock {
    /// Always `text_editor_code_execution_str_replace_result`.
    pub r#type: TextEditorCodeExecutionStrReplaceResultType,
    /// The surrounding lines, when reported.
    pub lines: Option<Vec<String>>,
    /// How many lines the new text spans.
    pub new_lines: Option<u64>,
    /// Where the new text starts.
    pub new_start: Option<u64>,
    /// How many lines the old text spanned.
    pub old_lines: Option<u64>,
    /// Where the old text started.
    pub old_start: Option<u64>,
}

/// The `text_editor_code_execution_str_replace_result` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum TextEditorCodeExecutionStrReplaceResultType {
    /// The only value.
    #[default]
    TextEditorCodeExecutionStrReplaceResult,
}

// ---------------------------------------------------------------
// Tool search
// ---------------------------------------------------------------

/// A tool search's content: references found, or the error shell.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum ToolSearchToolResultContent {
    /// The search failed.
    Error(ToolSearchToolResultError),
    /// What it found.
    SearchResult(ToolSearchToolSearchResultBlock),
}

/// A tool search's error shell.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct ToolSearchToolResultError {
    /// Always `tool_search_tool_result_error`.
    pub r#type: ToolSearchToolResultErrorType,
    /// Why it failed — an API-owned vocabulary, open.
    pub error_code: String,
    /// The failure in words, when the API says more.
    pub error_message: Option<String>,
}

/// The `tool_search_tool_result_error` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ToolSearchToolResultErrorType {
    /// The only value.
    #[default]
    ToolSearchToolResultError,
}

/// What a tool search found: references to tools.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct ToolSearchToolSearchResultBlock {
    /// Always `tool_search_tool_search_result`.
    pub r#type: ToolSearchToolSearchResultType,
    /// The tools found.
    pub tool_references: Vec<ToolReferenceBlock>,
}

/// The `tool_search_tool_search_result` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ToolSearchToolSearchResultType {
    /// The only value.
    #[default]
    ToolSearchToolSearchResult,
}

/// A reference to a tool, by name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct ToolReferenceBlock {
    /// Always `tool_reference`.
    pub r#type: ToolReferenceType,
    /// The tool's name.
    pub tool_name: String,
}

/// The `tool_reference` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ToolReferenceType {
    /// The only value.
    #[default]
    ToolReference,
}
