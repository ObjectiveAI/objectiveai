/// Returns JSON Schemas for all public types that derive `JsonSchema`.
///
/// Each schema is generated via `schemars::schema_for!()` and includes
/// the type's `$defs` for referenced types. The schema title matches
/// the `#[schemars(rename = "...")]` attribute on each type.
pub fn json_schemas() -> Vec<schemars::Schema> {
    let mut schemas = vec![
        schemars::schema_for!(crate::logs::IndexedLogReference),
        schemars::schema_for!(crate::logs::LogReference),
        schemars::schema_for!(crate::logs::LogReferenceTag),
        schemars::schema_for!(crate::remote::Remote),
        schemars::schema_for!(crate::remote::RemotePath),
        schemars::schema_for!(crate::remote::RemotePathCommitOptional),
        schemars::schema_for!(crate::weights::Weights),
        schemars::schema_for!(crate::weights::WeightsEntry),
        schemars::schema_for!(crate::agent::Agent),
        schemars::schema_for!(crate::agent::AgentBase),
        schemars::schema_for!(crate::agent::AgentWithFallbacks),
        schemars::schema_for!(crate::agent::AgentWithFallbacksWithCount),
        schemars::schema_for!(crate::agent::ClientObjectiveaiMcp),
        schemars::schema_for!(crate::agent::ClientObjectiveaiMcpEntry),
        schemars::schema_for!(crate::agent::ClientObjectiveaiMcpHeaders),
        schemars::schema_for!(crate::agent::ClientObjectiveaiMcpPluginEntry),
        schemars::schema_for!(crate::agent::ClientObjectiveaiMcpPluginMcpServer),
        schemars::schema_for!(crate::agent::Continuation),
        schemars::schema_for!(crate::agent::response::GetAgentResponse),
        schemars::schema_for!(crate::agent::InlineAgent),
        schemars::schema_for!(crate::agent::InlineAgentBase),
        schemars::schema_for!(crate::agent::InlineAgentBaseWithFallbacks),
        schemars::schema_for!(crate::agent::InlineAgentBaseWithFallbacksOrRemote),
        schemars::schema_for!(crate::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional),
        schemars::schema_for!(crate::agent::InlineAgentBaseWithFallbacksOrRemoteWithCount),
        schemars::schema_for!(crate::agent::InlineAgentWithFallbacks),
        schemars::schema_for!(crate::agent::response::ListAgentResponse),
        schemars::schema_for!(crate::agent::request::ListAgentsRequest),
        schemars::schema_for!(crate::agent::request::ListAgentsSource),
        schemars::schema_for!(crate::agent::McpServer),
        schemars::schema_for!(crate::agent::OutputMode),
        schemars::schema_for!(crate::agent::RemoteAgent),
        schemars::schema_for!(crate::agent::RemoteAgentBase),
        schemars::schema_for!(crate::agent::RemoteAgentBaseWithFallbacks),
        schemars::schema_for!(crate::agent::RemoteAgentWithFallbacks),
        schemars::schema_for!(crate::agent::Upstream),
        schemars::schema_for!(crate::agent::response::UsageAgentResponse),
        schemars::schema_for!(crate::agent::claude_agent_sdk::Agent),
        schemars::schema_for!(crate::agent::claude_agent_sdk::AgentBase),
        schemars::schema_for!(crate::agent::claude_agent_sdk::Continuation),
        schemars::schema_for!(crate::agent::claude_agent_sdk::Effort),
        schemars::schema_for!(crate::agent::claude_agent_sdk::OutputMode),
        schemars::schema_for!(crate::agent::claude_agent_sdk::upstream::Upstream),
        schemars::schema_for!(crate::agent::codex_sdk::Agent),
        schemars::schema_for!(crate::agent::codex_sdk::AgentBase),
        schemars::schema_for!(crate::agent::codex_sdk::Continuation),
        schemars::schema_for!(crate::agent::codex_sdk::Effort),
        schemars::schema_for!(crate::agent::codex_sdk::OutputMode),
        schemars::schema_for!(crate::agent::codex_sdk::upstream::Upstream),
        schemars::schema_for!(crate::agent::completions::message::AssistantMessage),
        schemars::schema_for!(crate::agent::completions::message::AssistantMessageExpression),
        schemars::schema_for!(crate::agent::completions::message::AssistantMessageLog),
        schemars::schema_for!(crate::agent::completions::message::AssistantToolCall),
        schemars::schema_for!(crate::agent::completions::message::AssistantToolCallDelta),
        schemars::schema_for!(crate::agent::completions::message::AssistantToolCallExpression),
        schemars::schema_for!(crate::agent::completions::message::AssistantToolCallFunction),
        schemars::schema_for!(crate::agent::completions::message::AssistantToolCallFunctionDelta),
        schemars::schema_for!(crate::agent::completions::message::AssistantToolCallFunctionExpression),
        schemars::schema_for!(crate::agent::completions::message::AssistantToolCallType),
        schemars::schema_for!(crate::agent::completions::message::DeveloperMessage),
        schemars::schema_for!(crate::agent::completions::message::DeveloperMessageExpression),
        schemars::schema_for!(crate::agent::completions::message::DeveloperMessageLog),
        schemars::schema_for!(crate::agent::completions::message::File),
        schemars::schema_for!(crate::agent::completions::message::ImageUrl),
        schemars::schema_for!(crate::agent::completions::message::ImageUrlDetail),
        schemars::schema_for!(crate::agent::completions::message::InputAudio),
        schemars::schema_for!(crate::agent::completions::message::Message),
        schemars::schema_for!(crate::agent::completions::message::MessageExpression),
        schemars::schema_for!(crate::agent::completions::message::MessageLog),
        schemars::schema_for!(crate::agent::completions::message::PipeAck),
        schemars::schema_for!(crate::agent::completions::message::RichContent),
        schemars::schema_for!(crate::agent::completions::message::RichContentExpression),
        schemars::schema_for!(crate::agent::completions::message::RichContentLog),
        schemars::schema_for!(crate::agent::completions::message::RichContentPart),
        schemars::schema_for!(crate::agent::completions::message::RichContentPartExpression),
        schemars::schema_for!(crate::agent::completions::message::SimpleContent),
        schemars::schema_for!(crate::agent::completions::message::SimpleContentExpression),
        schemars::schema_for!(crate::agent::completions::message::SimpleContentLog),
        schemars::schema_for!(crate::agent::completions::message::SimpleContentPart),
        schemars::schema_for!(crate::agent::completions::message::SimpleContentPartExpression),
        schemars::schema_for!(crate::agent::completions::message::SystemMessage),
        schemars::schema_for!(crate::agent::completions::message::SystemMessageExpression),
        schemars::schema_for!(crate::agent::completions::message::SystemMessageLog),
        schemars::schema_for!(crate::agent::completions::message::ToolMessage),
        schemars::schema_for!(crate::agent::completions::message::ToolMessageExpression),
        schemars::schema_for!(crate::agent::completions::message::ToolMessageLog),
        schemars::schema_for!(crate::agent::completions::message::ToolResponseMetadata),
        schemars::schema_for!(crate::agent::completions::message::UserMessage),
        schemars::schema_for!(crate::agent::completions::message::UserMessageExpression),
        schemars::schema_for!(crate::agent::completions::message::UserMessageLog),
        schemars::schema_for!(crate::agent::completions::message::VideoUrl),
        schemars::schema_for!(crate::agent::completions::request::AgentCompletionCreateParams),
        schemars::schema_for!(crate::agent::completions::request::AgentCompletionCreateParamsLog),
        schemars::schema_for!(crate::agent::completions::request::AgentCompletionNotifyParams),
        schemars::schema_for!(crate::agent::completions::request::Provider),
        schemars::schema_for!(crate::agent::completions::request::ProviderDataCollection),
        schemars::schema_for!(crate::agent::completions::request::ProviderMaxPrice),
        schemars::schema_for!(crate::agent::completions::request::ProviderSort),
        schemars::schema_for!(crate::agent::completions::request::ResponseFormat),
        schemars::schema_for!(crate::agent::completions::request::ResponseFormatParam),
        schemars::schema_for!(crate::agent::completions::response::AssistantRole),
        schemars::schema_for!(crate::agent::completions::response::CompletionTokensDetails),
        schemars::schema_for!(crate::agent::completions::response::CostDetails),
        schemars::schema_for!(crate::agent::completions::response::FinishReason),
        schemars::schema_for!(crate::agent::completions::response::Logprob),
        schemars::schema_for!(crate::agent::completions::response::Logprobs),
        schemars::schema_for!(crate::agent::completions::response::PromptTokensDetails),
        schemars::schema_for!(crate::agent::completions::response::ToolResponse),
        schemars::schema_for!(crate::agent::completions::response::ToolResponseLog),
        schemars::schema_for!(crate::agent::completions::response::ToolRole),
        schemars::schema_for!(crate::agent::completions::response::TopLogprob),
        schemars::schema_for!(crate::agent::completions::response::UpstreamUsage),
        schemars::schema_for!(crate::agent::completions::response::Usage),
        schemars::schema_for!(crate::agent::completions::response::streaming::AgentCompletionChunk),
        schemars::schema_for!(crate::agent::completions::response::streaming::AgentCompletionChunkLog),
        schemars::schema_for!(crate::agent::completions::response::streaming::AssistantResponseChunk),
        schemars::schema_for!(crate::agent::completions::response::streaming::AssistantResponseChunkLog),
        schemars::schema_for!(crate::agent::completions::response::streaming::MessageChunk),
        schemars::schema_for!(crate::agent::completions::response::streaming::message_log_reference::LogReference),
        schemars::schema_for!(crate::agent::completions::response::streaming::message_log_reference::Role),
        schemars::schema_for!(crate::agent::completions::response::streaming::Object),
        schemars::schema_for!(crate::agent::completions::response::unary::AgentCompletion),
        schemars::schema_for!(crate::agent::completions::response::unary::AssistantResponse),
        schemars::schema_for!(crate::agent::completions::response::unary::Message),
        schemars::schema_for!(crate::agent::completions::response::unary::Object),
        schemars::schema_for!(crate::agent::favorites::Action),
        schemars::schema_for!(crate::agent::favorites::ChangedNotification),
        schemars::schema_for!(crate::agent::mock::Agent),
        schemars::schema_for!(crate::agent::mock::AgentBase),
        schemars::schema_for!(crate::agent::mock::Call),
        schemars::schema_for!(crate::agent::mock::CallToolCall),
        schemars::schema_for!(crate::agent::mock::Continuation),
        schemars::schema_for!(crate::agent::mock::Mode),
        schemars::schema_for!(crate::agent::mock::OutputMode),
        schemars::schema_for!(crate::agent::mock::upstream::Upstream),
        schemars::schema_for!(crate::agent::openrouter::Agent),
        schemars::schema_for!(crate::agent::openrouter::AgentBase),
        schemars::schema_for!(crate::agent::openrouter::Continuation),
        schemars::schema_for!(crate::agent::openrouter::OutputMode),
        schemars::schema_for!(crate::agent::openrouter::Provider),
        schemars::schema_for!(crate::agent::openrouter::ContextCompression),
        schemars::schema_for!(crate::agent::openrouter::ProviderQuantization),
        schemars::schema_for!(crate::agent::openrouter::Reasoning),
        schemars::schema_for!(crate::agent::openrouter::ReasoningEffort),
        schemars::schema_for!(crate::agent::openrouter::ReasoningSummaryVerbosity),
        schemars::schema_for!(crate::agent::openrouter::Stop),
        schemars::schema_for!(crate::agent::openrouter::upstream::Upstream),
        schemars::schema_for!(crate::agent::openrouter::Verbosity),
        schemars::schema_for!(crate::auth::ApiKeyWithMetadata),
        schemars::schema_for!(crate::auth::request::CreateApiKeyRequest),
        schemars::schema_for!(crate::auth::request::CreateOpenRouterByokApiKeyRequest),
        schemars::schema_for!(crate::auth::request::DisableApiKeyRequest),
        schemars::schema_for!(crate::auth::response::GetCreditsResponse),
        schemars::schema_for!(crate::auth::response::GetOpenRouterByokApiKeyResponse),
        schemars::schema_for!(crate::auth::response::ListApiKeyItem),
        schemars::schema_for!(crate::auth::response::ListApiKeyResponse),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::Error),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::ErrorType),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::Level),
        #[cfg(feature = "cli-executor")]
        schemars::schema_for!(crate::cli::command::AgentArguments),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::Ok),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::RemotePathCommitOptionalOrFavorite),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::list::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::list::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::list::active::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::list::active::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::list::active::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::list::active::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::list::active::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::list::active::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::list::active::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::list::available::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::list::available::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::list::available::RequestSource),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::list::available::ResponseFavorite),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::list::available::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::list::available::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::list::available::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::list::available::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::list::available::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::me::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::me::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::me::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::me::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::me::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::me::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::me::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message::MessageTarget),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message::RequestDangerousAdvanced),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message::RequestMessage),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message_queue::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message_queue::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message_queue::add::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message_queue::add::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message_queue::add::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message_queue::add::Target),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message_queue::add::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message_queue::add::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message_queue::add::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message_queue::add::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message_queue::delete::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message_queue::delete::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message_queue::delete::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message_queue::delete::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message_queue::delete::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message_queue::delete::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message_queue::delete::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message_queue::deliver::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message_queue::deliver::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message_queue::deliver::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message_queue::deliver::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message_queue::deliver::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message_queue::deliver::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message_queue::deliver::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message_queue::read::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message_queue::read::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message_queue::read::id::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message_queue::read::id::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message_queue::read::id::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message_queue::read::id::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message_queue::read::id::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message_queue::read::id::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message_queue::read::pending::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message_queue::read::pending::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message_queue::read::pending::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message_queue::read::pending::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message_queue::read::pending::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message_queue::read::pending::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::message_queue::read::pending::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::publish::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::publish::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::publish::RequestBody),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::publish::RequestPublishMessage),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::publish::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::publish::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::publish::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::publish::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::publish::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::read::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::read::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::read::all::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::read::all::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::read::all::ResponseContent),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::read::all::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::read::all::ResponseQueueItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::read::all::ResponseQueueMessage),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::read::all::Target),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::read::all::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::read::all::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::read::all::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::read::all::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::read::id::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::read::id::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::read::id::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::read::id::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::read::id::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::read::id::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::read::id::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::read::pending::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::read::pending::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::read::pending::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::read::pending::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::read::pending::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::read::pending::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::read::pending::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::read::subscribe::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::read::subscribe::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::read::subscribe::RequestMessageKind),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::read::subscribe::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::read::subscribe::SubscribeTarget),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::read::subscribe::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::read::subscribe::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::read::subscribe::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::read::subscribe::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::spawn::AgentSpec),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::spawn::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::spawn::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::spawn::RequestDangerousAdvanced),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::spawn::RequestPrompt),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::spawn::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::spawn::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::spawn::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::spawn::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::spawn::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::tags::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::tags::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::tags::add::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::tags::add::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::tags::add::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::tags::add::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::tags::add::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::tags::add::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::tags::add::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::tags::lookup::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::tags::lookup::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::tags::lookup::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::tags::lookup::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::tags::lookup::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::tags::lookup::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::tags::lookup::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::tasks::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::tasks::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::tasks::list::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::tasks::list::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::tasks::list::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::tasks::list::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::tasks::list::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::tasks::list::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::tasks::list::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::tasks::run::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::tasks::run::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::tasks::run::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::tasks::run::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::tasks::run::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::tasks::run::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::tasks::run::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::tasks::schedule::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::tasks::schedule::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::tasks::schedule::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::tasks::schedule::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::tasks::schedule::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::tasks::schedule::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::tasks::schedule::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::agents::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::agents::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::agents::favorites::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::agents::favorites::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::agents::favorites::add::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::agents::favorites::add::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::agents::favorites::add::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::agents::favorites::add::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::agents::favorites::add::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::agents::favorites::add::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::agents::favorites::del::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::agents::favorites::del::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::agents::favorites::del::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::agents::favorites::del::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::agents::favorites::del::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::agents::favorites::del::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::agents::favorites::edit::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::agents::favorites::edit::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::agents::favorites::edit::RequestCommitChange),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::agents::favorites::edit::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::agents::favorites::edit::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::agents::favorites::edit::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::agents::favorites::edit::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::agents::favorites::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::agents::favorites::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::agents::favorites::get::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::agents::favorites::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::agents::favorites::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::agents::favorites::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::agents::favorites::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::agents::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::agents::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::agents::get::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::agents::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::agents::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::agents::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::agents::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::favorites::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::favorites::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::favorites::add::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::favorites::add::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::favorites::add::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::favorites::add::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::favorites::add::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::favorites::add::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::favorites::del::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::favorites::del::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::favorites::del::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::favorites::del::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::favorites::del::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::favorites::del::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::favorites::edit::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::favorites::edit::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::favorites::edit::RequestCommitChange),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::favorites::edit::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::favorites::edit::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::favorites::edit::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::favorites::edit::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::favorites::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::favorites::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::favorites::get::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::favorites::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::favorites::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::favorites::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::favorites::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::get::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::inventions::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::inventions::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::inventions::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::inventions::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::inventions::get::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::inventions::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::inventions::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::inventions::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::inventions::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::inventions::remote::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::inventions::remote::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::inventions::remote::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::inventions::remote::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::inventions::remote::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::inventions::remote::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::inventions::remote::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::inventions::remote::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::inventions::remote::set::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::inventions::remote::set::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::inventions::remote::set::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::inventions::remote::set::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::inventions::remote::set::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::inventions::remote::set::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::favorites::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::favorites::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::favorites::add::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::favorites::add::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::favorites::add::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::favorites::add::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::favorites::add::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::favorites::add::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::favorites::del::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::favorites::del::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::favorites::del::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::favorites::del::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::favorites::del::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::favorites::del::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::favorites::edit::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::favorites::edit::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::favorites::edit::RequestCommitChange),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::favorites::edit::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::favorites::edit::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::favorites::edit::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::favorites::edit::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::favorites::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::favorites::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::favorites::get::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::favorites::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::favorites::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::favorites::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::favorites::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::get::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::pairs::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::pairs::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::pairs::favorites::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::pairs::favorites::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::pairs::favorites::add::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::pairs::favorites::add::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::pairs::favorites::add::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::pairs::favorites::add::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::pairs::favorites::add::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::pairs::favorites::add::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::pairs::favorites::del::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::pairs::favorites::del::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::pairs::favorites::del::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::pairs::favorites::del::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::pairs::favorites::del::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::pairs::favorites::del::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::pairs::favorites::edit::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::pairs::favorites::edit::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::pairs::favorites::edit::RequestCommitChange),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::pairs::favorites::edit::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::pairs::favorites::edit::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::pairs::favorites::edit::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::pairs::favorites::edit::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::pairs::favorites::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::pairs::favorites::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::pairs::favorites::get::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::pairs::favorites::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::pairs::favorites::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::pairs::favorites::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::pairs::favorites::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::pairs::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::pairs::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::pairs::get::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::pairs::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::pairs::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::pairs::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::pairs::get::response_schema::Request),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::mcp::Request),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::mcp::Response),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::mcp::address::Request),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::mcp::address::Response),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::mcp::address::get::Path),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::mcp::address::get::Request),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::mcp::address::get::Response),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::mcp::address::get::request_schema::Path),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::mcp::address::get::request_schema::Request),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::mcp::address::get::response_schema::Path),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::mcp::address::get::response_schema::Request),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::mcp::address::set::Path),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::mcp::address::set::Request),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::mcp::address::set::request_schema::Path),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::mcp::address::set::request_schema::Request),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::mcp::address::set::response_schema::Path),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::mcp::address::set::response_schema::Request),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::mcp::get::Path),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::mcp::get::Request),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::mcp::get::Response),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::mcp::get::request_schema::Path),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::mcp::get::request_schema::Request),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::mcp::get::response_schema::Path),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::mcp::get::response_schema::Request),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::mcp::port::Request),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::mcp::port::Response),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::mcp::port::get::Path),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::mcp::port::get::Request),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::mcp::port::get::Response),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::mcp::port::get::request_schema::Path),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::mcp::port::get::request_schema::Request),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::mcp::port::get::response_schema::Path),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::mcp::port::get::response_schema::Request),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::mcp::port::set::Path),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::mcp::port::set::Request),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::mcp::port::set::request_schema::Path),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::mcp::port::set::request_schema::Request),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::mcp::port::set::response_schema::Path),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::mcp::port::set::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::swarms::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::swarms::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::swarms::favorites::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::swarms::favorites::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::swarms::favorites::add::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::swarms::favorites::add::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::swarms::favorites::add::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::swarms::favorites::add::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::swarms::favorites::add::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::swarms::favorites::add::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::swarms::favorites::del::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::swarms::favorites::del::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::swarms::favorites::del::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::swarms::favorites::del::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::swarms::favorites::del::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::swarms::favorites::del::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::swarms::favorites::edit::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::swarms::favorites::edit::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::swarms::favorites::edit::RequestCommitChange),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::swarms::favorites::edit::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::swarms::favorites::edit::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::swarms::favorites::edit::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::swarms::favorites::edit::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::swarms::favorites::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::swarms::favorites::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::swarms::favorites::get::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::swarms::favorites::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::swarms::favorites::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::swarms::favorites::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::swarms::favorites::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::swarms::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::swarms::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::swarms::get::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::swarms::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::swarms::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::swarms::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::swarms::get::response_schema::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::Response),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::address::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::address::Response),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::address::get::Path),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::address::get::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::address::get::Response),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::address::get::request_schema::Path),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::address::get::request_schema::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::address::get::response_schema::Path),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::address::get::response_schema::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::address::set::Path),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::address::set::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::address::set::request_schema::Path),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::address::set::request_schema::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::address::set::response_schema::Path),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::address::set::response_schema::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::get::Path),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::get::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::get::Response),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::get::request_schema::Path),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::get::request_schema::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::get::response_schema::Path),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::get::response_schema::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::port::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::port::Response),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::port::get::Path),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::port::get::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::port::get::Response),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::port::get::request_schema::Path),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::port::get::request_schema::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::port::get::response_schema::Path),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::port::get::response_schema::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::port::set::Path),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::port::set::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::port::set::request_schema::Path),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::port::set::request_schema::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::port::set::response_schema::Path),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::port::set::response_schema::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::secret::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::secret::Response),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::secret::get::Path),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::secret::get::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::secret::get::Response),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::secret::get::request_schema::Path),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::secret::get::request_schema::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::secret::get::response_schema::Path),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::secret::get::response_schema::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::secret::set::Path),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::secret::set::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::secret::set::request_schema::Path),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::secret::set::request_schema::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::secret::set::response_schema::Path),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::secret::set::response_schema::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::signature::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::signature::Response),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::signature::get::Path),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::signature::get::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::signature::get::Response),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::signature::get::request_schema::Path),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::signature::get::request_schema::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::signature::get::response_schema::Path),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::signature::get::response_schema::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::signature::set::Path),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::signature::set::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::signature::set::request_schema::Path),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::signature::set::request_schema::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::signature::set::response_schema::Path),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::signature::set::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::executions::create::FunctionSpec),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::executions::create::ProfileSpec),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::executions::create::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::executions::create::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::executions::create::standard::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::executions::create::standard::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::executions::create::standard::RequestDangerousAdvanced),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::executions::create::standard::RequestInput),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::executions::create::standard::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::executions::create::standard::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::executions::create::standard::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::executions::create::standard::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::executions::create::standard::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::executions::create::swiss_system::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::executions::create::swiss_system::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::executions::create::swiss_system::RequestDangerousAdvanced),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::executions::create::swiss_system::RequestInput),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::executions::create::swiss_system::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::executions::create::swiss_system::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::executions::create::swiss_system::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::executions::create::swiss_system::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::executions::create::swiss_system::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::inventions::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::inventions::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::inventions::recursive::create::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::inventions::recursive::create::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::inventions::recursive::create::alpha_scalar::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::inventions::recursive::create::alpha_scalar::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::inventions::recursive::create::alpha_scalar::RequestDangerousAdvanced),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::inventions::recursive::create::alpha_scalar::RequestParams),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::inventions::recursive::create::alpha_scalar::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::inventions::recursive::create::alpha_scalar::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::inventions::recursive::create::alpha_scalar::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::inventions::recursive::create::alpha_scalar::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::inventions::recursive::create::alpha_scalar::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::inventions::recursive::create::alpha_vector::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::inventions::recursive::create::alpha_vector::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::inventions::recursive::create::alpha_vector::RequestDangerousAdvanced),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::inventions::recursive::create::alpha_vector::RequestParams),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::inventions::recursive::create::alpha_vector::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::inventions::recursive::create::alpha_vector::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::inventions::recursive::create::alpha_vector::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::inventions::recursive::create::alpha_vector::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::inventions::recursive::create::alpha_vector::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::inventions::recursive::create::remote::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::inventions::recursive::create::remote::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::inventions::recursive::create::remote::RequestDangerousAdvanced),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::inventions::recursive::create::remote::RequestState),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::inventions::recursive::create::remote::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::inventions::recursive::create::remote::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::inventions::recursive::create::remote::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::inventions::recursive::create::remote::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::inventions::recursive::create::remote::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::inventions::state::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::inventions::state::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::inventions::state::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::inventions::state::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::inventions::state::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::inventions::state::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::inventions::state::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::inventions::state::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::list::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::list::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::list::RequestSource),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::list::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::list::ResponseFavorite),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::list::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::list::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::list::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::list::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::list::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::profiles::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::profiles::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::profiles::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::profiles::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::profiles::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::profiles::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::profiles::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::profiles::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::profiles::list::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::profiles::list::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::profiles::list::RequestSource),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::profiles::list::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::profiles::list::ResponseFavorite),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::profiles::list::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::profiles::list::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::profiles::list::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::profiles::list::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::profiles::list::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::profiles::publish::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::profiles::publish::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::profiles::publish::RequestBody),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::profiles::publish::RequestPublishMessage),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::profiles::publish::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::profiles::publish::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::profiles::publish::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::profiles::publish::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::profiles::publish::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::publish::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::publish::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::publish::RequestBody),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::publish::RequestPublishMessage),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::publish::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::publish::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::publish::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::publish::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::publish::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::messages::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::messages::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::messages::audio::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::messages::audio::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::messages::audio::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::messages::audio::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::messages::audio::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::messages::audio::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::messages::audio::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::messages::audio::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::messages::file::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::messages::file::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::messages::file::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::messages::file::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::messages::file::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::messages::file::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::messages::file::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::messages::file::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::messages::image::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::messages::image::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::messages::image::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::messages::image::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::messages::image::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::messages::image::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::messages::image::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::messages::image::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::messages::text::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::messages::text::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::messages::text::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::messages::text::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::messages::text::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::messages::text::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::messages::text::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::messages::text::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::messages::video::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::messages::video::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::messages::video::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::messages::video::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::messages::video::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::messages::video::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::messages::video::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::messages::video::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::notifications::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::notifications::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::notifications::audio::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::notifications::audio::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::notifications::audio::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::notifications::audio::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::notifications::audio::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::notifications::audio::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::notifications::audio::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::notifications::audio::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::notifications::file::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::notifications::file::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::notifications::file::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::notifications::file::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::notifications::file::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::notifications::file::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::notifications::file::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::notifications::file::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::notifications::image::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::notifications::image::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::notifications::image::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::notifications::image::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::notifications::image::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::notifications::image::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::notifications::image::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::notifications::image::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::notifications::text::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::notifications::text::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::notifications::text::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::notifications::text::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::notifications::text::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::notifications::text::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::notifications::text::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::notifications::text::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::notifications::video::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::notifications::video::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::notifications::video::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::notifications::video::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::notifications::video::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::notifications::video::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::notifications::video::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::notifications::video::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::subscribe::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::subscribe::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::subscribe::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::subscribe::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::subscribe::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::request::subscribe::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::clear::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::clear::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::clear::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::clear::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::clear::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::clear::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::clear::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::continuations::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::continuations::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::continuations::clear::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::continuations::clear::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::continuations::clear::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::continuations::clear::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::continuations::clear::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::continuations::clear::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::continuations::clear::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::continuations::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::continuations::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::continuations::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::continuations::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::continuations::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::continuations::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::continuations::subscribe::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::continuations::subscribe::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::continuations::subscribe::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::continuations::subscribe::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::continuations::subscribe::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::continuations::subscribe::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::list::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::list::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::list::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::list::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::list::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::list::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::list::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::audio::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::audio::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::audio::clear::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::audio::clear::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::audio::clear::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::audio::clear::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::audio::clear::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::audio::clear::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::audio::clear::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::audio::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::audio::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::audio::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::audio::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::audio::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::audio::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::audio::subscribe::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::audio::subscribe::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::audio::subscribe::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::audio::subscribe::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::audio::subscribe::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::audio::subscribe::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::clear::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::clear::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::clear::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::clear::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::clear::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::clear::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::clear::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::file::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::file::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::file::clear::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::file::clear::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::file::clear::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::file::clear::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::file::clear::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::file::clear::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::file::clear::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::file::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::file::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::file::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::file::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::file::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::file::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::file::subscribe::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::file::subscribe::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::file::subscribe::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::file::subscribe::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::file::subscribe::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::file::subscribe::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::image::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::image::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::image::clear::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::image::clear::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::image::clear::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::image::clear::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::image::clear::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::image::clear::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::image::clear::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::image::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::image::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::image::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::image::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::image::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::image::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::image::subscribe::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::image::subscribe::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::image::subscribe::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::image::subscribe::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::image::subscribe::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::image::subscribe::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::logprobs::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::logprobs::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::logprobs::clear::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::logprobs::clear::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::logprobs::clear::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::logprobs::clear::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::logprobs::clear::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::logprobs::clear::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::logprobs::clear::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::logprobs::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::logprobs::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::logprobs::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::logprobs::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::logprobs::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::logprobs::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::logprobs::subscribe::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::logprobs::subscribe::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::logprobs::subscribe::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::logprobs::subscribe::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::logprobs::subscribe::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::logprobs::subscribe::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::reasoning::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::reasoning::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::reasoning::clear::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::reasoning::clear::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::reasoning::clear::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::reasoning::clear::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::reasoning::clear::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::reasoning::clear::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::reasoning::clear::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::reasoning::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::reasoning::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::reasoning::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::reasoning::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::reasoning::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::reasoning::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::reasoning::subscribe::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::reasoning::subscribe::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::reasoning::subscribe::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::reasoning::subscribe::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::reasoning::subscribe::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::reasoning::subscribe::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::refusal::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::refusal::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::refusal::clear::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::refusal::clear::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::refusal::clear::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::refusal::clear::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::refusal::clear::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::refusal::clear::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::refusal::clear::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::refusal::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::refusal::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::refusal::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::refusal::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::refusal::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::refusal::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::refusal::subscribe::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::refusal::subscribe::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::refusal::subscribe::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::refusal::subscribe::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::refusal::subscribe::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::refusal::subscribe::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::subscribe::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::subscribe::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::subscribe::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::subscribe::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::subscribe::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::subscribe::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::text::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::text::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::text::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::text::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::text::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::text::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::text::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::text::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::clear::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::clear::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::clear::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::clear::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::clear::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::clear::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::clear::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::subscribe::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::subscribe::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::subscribe::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::subscribe::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::subscribe::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::subscribe::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::audio::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::audio::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::audio::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::audio::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::audio::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::audio::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::audio::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::audio::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::file::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::file::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::file::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::file::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::file::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::file::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::file::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::file::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::image::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::image::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::image::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::image::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::image::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::image::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::image::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::image::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::text::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::text::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::text::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::text::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::text::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::text::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::text::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::text::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::video::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::video::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::video::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::video::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::video::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::video::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::video::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::tool::video::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::tool_calls::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::tool_calls::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::tool_calls::clear::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::tool_calls::clear::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::tool_calls::clear::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::tool_calls::clear::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::tool_calls::clear::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::tool_calls::clear::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::tool_calls::clear::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::tool_calls::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::tool_calls::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::tool_calls::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::tool_calls::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::tool_calls::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::tool_calls::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::tool_calls::subscribe::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::tool_calls::subscribe::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::tool_calls::subscribe::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::tool_calls::subscribe::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::tool_calls::subscribe::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::tool_calls::subscribe::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::video::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::video::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::video::clear::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::video::clear::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::video::clear::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::video::clear::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::video::clear::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::video::clear::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::video::clear::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::video::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::video::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::video::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::video::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::video::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::video::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::video::subscribe::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::video::subscribe::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::video::subscribe::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::video::subscribe::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::video::subscribe::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::messages::assistant::video::subscribe::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::subscribe::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::subscribe::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::subscribe::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::subscribe::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::subscribe::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::agents::completions::response::subscribe::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::clear::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::clear::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::clear::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::clear::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::clear::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::clear::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::clear::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::request::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::request::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::request::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::request::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::request::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::request::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::request::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::request::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::request::subscribe::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::request::subscribe::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::request::subscribe::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::request::subscribe::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::request::subscribe::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::request::subscribe::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::clear::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::clear::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::clear::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::clear::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::clear::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::clear::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::clear::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::list::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::list::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::list::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::list::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::list::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::list::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::list::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::retry_tokens::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::retry_tokens::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::retry_tokens::clear::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::retry_tokens::clear::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::retry_tokens::clear::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::retry_tokens::clear::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::retry_tokens::clear::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::retry_tokens::clear::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::retry_tokens::clear::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::retry_tokens::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::retry_tokens::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::retry_tokens::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::retry_tokens::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::retry_tokens::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::retry_tokens::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::retry_tokens::subscribe::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::retry_tokens::subscribe::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::retry_tokens::subscribe::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::retry_tokens::subscribe::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::retry_tokens::subscribe::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::retry_tokens::subscribe::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::subscribe::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::subscribe::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::subscribe::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::subscribe::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::subscribe::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::executions::response::subscribe::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::request::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::request::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::request::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::request::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::request::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::request::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::request::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::request::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::request::subscribe::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::request::subscribe::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::request::subscribe::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::request::subscribe::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::request::subscribe::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::request::subscribe::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::response::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::response::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::response::clear::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::response::clear::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::response::clear::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::response::clear::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::response::clear::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::response::clear::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::response::clear::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::response::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::response::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::response::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::response::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::response::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::response::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::response::list::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::response::list::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::response::list::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::response::list::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::response::list::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::response::list::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::response::list::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::response::subscribe::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::response::subscribe::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::response::subscribe::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::response::subscribe::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::response::subscribe::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::recursive::response::subscribe::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::request::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::request::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::request::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::request::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::request::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::request::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::request::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::request::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::request::subscribe::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::request::subscribe::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::request::subscribe::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::request::subscribe::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::request::subscribe::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::request::subscribe::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::response::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::response::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::response::clear::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::response::clear::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::response::clear::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::response::clear::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::response::clear::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::response::clear::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::response::clear::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::response::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::response::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::response::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::response::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::response::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::response::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::response::list::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::response::list::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::response::list::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::response::list::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::response::list::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::response::list::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::response::list::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::response::subscribe::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::response::subscribe::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::response::subscribe::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::response::subscribe::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::response::subscribe::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::functions::inventions::response::subscribe::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::request::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::request::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::request::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::request::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::request::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::request::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::request::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::request::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::request::subscribe::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::request::subscribe::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::request::subscribe::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::request::subscribe::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::request::subscribe::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::request::subscribe::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::response::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::response::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::response::clear::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::response::clear::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::response::clear::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::response::clear::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::response::clear::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::response::clear::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::response::clear::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::response::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::response::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::response::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::response::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::response::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::response::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::response::list::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::response::list::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::response::list::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::response::list::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::response::list::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::response::list::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::response::list::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::response::subscribe::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::response::subscribe::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::response::subscribe::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::response::subscribe::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::response::subscribe::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::logs::vector::completions::response::subscribe::response_schema::Request),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::mcp::Request),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::mcp::Response),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::mcp::kill::Path),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::mcp::kill::Request),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::mcp::kill::Response),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::mcp::kill::request_schema::Path),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::mcp::kill::request_schema::Request),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::mcp::kill::response_schema::Path),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::mcp::kill::response_schema::Request),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::mcp::spawn::Path),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::mcp::spawn::Request),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::mcp::spawn::Response),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::mcp::spawn::request_schema::Path),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::mcp::spawn::request_schema::Request),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::mcp::spawn::response_schema::Path),
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::mcp::spawn::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::get::ResponseBinaries),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::get::ResponseHttpMethod),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::get::ResponseManifest),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::get::ResponseMcpServer),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::get::ResponseViewerRoute),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::install::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::install::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::install::filesystem::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::install::filesystem::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::install::filesystem::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::install::filesystem::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::install::filesystem::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::install::filesystem::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::install::filesystem::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::install::github::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::install::github::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::install::github::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::install::github::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::install::github::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::install::github::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::install::github::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::list::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::list::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::list::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::list::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::list::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::list::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::run::Mcp),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::run::McpType),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::run::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::run::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::run::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::run::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::run::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::run::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::plugins::run::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::swarms::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::swarms::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::swarms::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::swarms::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::swarms::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::swarms::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::swarms::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::swarms::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::swarms::list::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::swarms::list::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::swarms::list::RequestSource),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::swarms::list::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::swarms::list::ResponseFavorite),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::swarms::list::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::swarms::list::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::swarms::list::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::swarms::list::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::swarms::list::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::swarms::publish::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::swarms::publish::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::swarms::publish::RequestBody),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::swarms::publish::RequestPublishMessage),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::swarms::publish::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::swarms::publish::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::swarms::publish::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::swarms::publish::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::swarms::publish::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::tools::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::tools::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::tools::get::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::tools::get::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::tools::get::ResponseManifest),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::tools::get::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::tools::get::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::tools::get::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::tools::get::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::tools::install::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::tools::install::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::tools::install::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::tools::install::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::tools::install::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::tools::install::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::tools::install::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::tools::list::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::tools::list::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::tools::list::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::tools::list::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::tools::list::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::tools::list::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::tools::run::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::tools::run::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::tools::run::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::tools::run::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::tools::run::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::tools::run::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::tools::run::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::update::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::update::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::update::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::update::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::update::ResponseSkipReason),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::update::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::update::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::update::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::update::response_schema::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::viewer::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::viewer::Response),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::viewer::generate_secret_signature_pair::Path),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::viewer::generate_secret_signature_pair::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::viewer::generate_secret_signature_pair::Response),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::viewer::generate_secret_signature_pair::request_schema::Path),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::viewer::generate_secret_signature_pair::request_schema::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::viewer::generate_secret_signature_pair::response_schema::Path),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::viewer::generate_secret_signature_pair::response_schema::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::viewer::kill::Path),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::viewer::kill::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::viewer::kill::Response),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::viewer::kill::request_schema::Path),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::viewer::kill::request_schema::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::viewer::kill::response_schema::Path),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::viewer::kill::response_schema::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::viewer::send::Path),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::viewer::send::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::viewer::send::Response),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::viewer::send::request_schema::Path),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::viewer::send::request_schema::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::viewer::send::response_schema::Path),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::viewer::send::response_schema::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::viewer::spawn::Path),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::viewer::spawn::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::viewer::spawn::Response),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::viewer::spawn::request_schema::Path),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::viewer::spawn::request_schema::Request),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::viewer::spawn::response_schema::Path),
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::viewer::spawn::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::plugins::Command),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::plugins::CommandType),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::plugins::Output),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::client_objectiveai_mcp::McpKind),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::client_objectiveai_mcp::client_request::McpListChanged),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::client_objectiveai_mcp::client_request::McpListChangedKind),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::client_objectiveai_mcp::client_request::Payload),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::client_objectiveai_mcp::client_request::Request),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::client_objectiveai_mcp::client_response::Response),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::client_objectiveai_mcp::server_request::InitializeRequest),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::client_objectiveai_mcp::server_request::Payload),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::client_objectiveai_mcp::server_request::Request),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::client_objectiveai_mcp::server_response::InitializeReply),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::client_objectiveai_mcp::server_response::Payload),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::client_objectiveai_mcp::server_response::Response),
        schemars::schema_for!(crate::error::request::ErrorCreateParams),
        schemars::schema_for!(crate::error::response::ErrorResponse),
        schemars::schema_for!(crate::error::ResponseError),
        schemars::schema_for!(crate::functions::AlphaInlineFunction),
        schemars::schema_for!(crate::functions::AlphaRemoteFunction),
        schemars::schema_for!(crate::functions::CompiledTask),
        schemars::schema_for!(crate::functions::FullFunction),
        schemars::schema_for!(crate::functions::FullInlineFunction),
        schemars::schema_for!(crate::functions::FullInlineFunctionOrRemoteCommitOptional),
        schemars::schema_for!(crate::functions::FullRemoteFunction),
        schemars::schema_for!(crate::functions::Function),
        schemars::schema_for!(crate::functions::FunctionType),
        schemars::schema_for!(crate::functions::response::GetFunctionProfilePairResponse),
        schemars::schema_for!(crate::functions::request::GetFunctionProfilePairUsageRequest),
        schemars::schema_for!(crate::functions::response::GetFunctionResponse),
        schemars::schema_for!(crate::functions::InlineFunction),
        schemars::schema_for!(crate::functions::InlineProfile),
        schemars::schema_for!(crate::functions::InlineProfileOrRemoteCommitOptional),
        schemars::schema_for!(crate::functions::InlineTasksProfile),
        schemars::schema_for!(crate::functions::response::ListFunctionProfilePairItem),
        schemars::schema_for!(crate::functions::response::ListFunctionProfilePairResponse),
        schemars::schema_for!(crate::functions::request::ListFunctionProfilePairsRequest),
        schemars::schema_for!(crate::functions::request::ListFunctionProfilePairsSource),
        schemars::schema_for!(crate::functions::response::ListFunctionResponse),
        schemars::schema_for!(crate::functions::request::ListFunctionsRequest),
        schemars::schema_for!(crate::functions::request::ListFunctionsSource),
        schemars::schema_for!(crate::functions::PlaceholderScalarFunctionTask),
        schemars::schema_for!(crate::functions::PlaceholderScalarFunctionTaskExpression),
        schemars::schema_for!(crate::functions::PlaceholderVectorFunctionTask),
        schemars::schema_for!(crate::functions::PlaceholderVectorFunctionTaskExpression),
        schemars::schema_for!(crate::functions::Profile),
        schemars::schema_for!(crate::functions::RemoteFunction),
        schemars::schema_for!(crate::functions::RemoteProfile),
        schemars::schema_for!(crate::functions::RemoteTasksProfile),
        schemars::schema_for!(crate::functions::ScalarFunctionTask),
        schemars::schema_for!(crate::functions::ScalarFunctionTaskExpression),
        schemars::schema_for!(crate::functions::Task),
        schemars::schema_for!(crate::functions::TaskExpression),
        schemars::schema_for!(crate::functions::TaskProfile),
        schemars::schema_for!(crate::functions::response::UsageFunctionProfilePairResponse),
        schemars::schema_for!(crate::functions::response::UsageFunctionResponse),
        schemars::schema_for!(crate::functions::VectorCompletionTask),
        schemars::schema_for!(crate::functions::VectorCompletionTaskExpression),
        schemars::schema_for!(crate::functions::VectorFunctionTask),
        schemars::schema_for!(crate::functions::VectorFunctionTaskExpression),
        schemars::schema_for!(crate::functions::alpha_scalar::BranchTaskExpression),
        schemars::schema_for!(crate::functions::alpha_scalar::InlineFunction),
        schemars::schema_for!(crate::functions::alpha_scalar::LeafTaskExpression),
        schemars::schema_for!(crate::functions::alpha_scalar::PartialPlaceholderBranchTaskExpression),
        schemars::schema_for!(crate::functions::alpha_scalar::PartialPlaceholderScalarFunctionTaskExpression),
        schemars::schema_for!(crate::functions::alpha_scalar::PlaceholderScalarFunctionTaskExpression),
        schemars::schema_for!(crate::functions::alpha_scalar::RemoteFunction),
        schemars::schema_for!(crate::functions::alpha_scalar::ScalarFunctionTaskExpression),
        schemars::schema_for!(crate::functions::alpha_scalar::VectorCompletionTaskExpression),
        schemars::schema_for!(crate::functions::alpha_vector::BranchTaskExpression),
        schemars::schema_for!(crate::functions::alpha_vector::InlineFunction),
        schemars::schema_for!(crate::functions::alpha_vector::LeafTaskExpression),
        schemars::schema_for!(crate::functions::alpha_vector::PartialPlaceholderBranchTaskExpression),
        schemars::schema_for!(crate::functions::alpha_vector::PartialPlaceholderScalarFunctionTaskExpression),
        schemars::schema_for!(crate::functions::alpha_vector::PartialPlaceholderVectorFunctionTaskExpression),
        schemars::schema_for!(crate::functions::alpha_vector::PlaceholderScalarFunctionTaskExpression),
        schemars::schema_for!(crate::functions::alpha_vector::PlaceholderVectorFunctionTaskExpression),
        schemars::schema_for!(crate::functions::alpha_vector::RemoteFunction),
        schemars::schema_for!(crate::functions::alpha_vector::ScalarFunctionTaskExpression),
        schemars::schema_for!(crate::functions::alpha_vector::VectorCompletionTaskExpression),
        schemars::schema_for!(crate::functions::alpha_vector::VectorFunctionTaskExpression),
        schemars::schema_for!(crate::functions::alpha_vector::expression::VectorFunctionInputSchema),
        schemars::schema_for!(crate::functions::alpha_vector::expression::VectorFunctionInputValue),
        schemars::schema_for!(crate::functions::alpha_vector::expression::VectorFunctionInputValueExpression),
        schemars::schema_for!(crate::functions::check::ScalarFieldsValidation),
        schemars::schema_for!(crate::functions::check::VectorFieldsValidation),
        schemars::schema_for!(crate::functions::executions::RetryToken),
        schemars::schema_for!(crate::functions::executions::request::FunctionExecutionCreateParams),
        schemars::schema_for!(crate::functions::executions::request::FunctionExecutionCreateParamsLog),
        schemars::schema_for!(crate::functions::executions::request::Reasoning),
        schemars::schema_for!(crate::functions::executions::request::Strategy),
        schemars::schema_for!(crate::functions::executions::response::Output),
        schemars::schema_for!(crate::functions::executions::response::streaming::FunctionExecutionChunk),
        schemars::schema_for!(crate::functions::executions::response::streaming::FunctionExecutionChunkLog),
        schemars::schema_for!(crate::functions::executions::response::streaming::FunctionExecutionTaskChunk),
        schemars::schema_for!(crate::functions::executions::response::streaming::InnerError<'_>),
        schemars::schema_for!(crate::functions::executions::response::streaming::Object),
        schemars::schema_for!(crate::functions::executions::response::streaming::ReasoningSummaryChunk),
        schemars::schema_for!(crate::functions::executions::response::streaming::TaskChunk),
        schemars::schema_for!(crate::functions::executions::response::streaming::VectorCompletionTaskChunk),
        schemars::schema_for!(crate::functions::executions::response::streaming::function_execution_task_log_reference::LogReference),
        schemars::schema_for!(crate::functions::executions::response::streaming::reasoning_summary_log_reference::LogReference),
        schemars::schema_for!(crate::functions::executions::response::streaming::task_log_reference::LogReference),
        schemars::schema_for!(crate::functions::executions::response::streaming::vector_completion_task_log_reference::LogReference),
        schemars::schema_for!(crate::functions::executions::response::unary::FunctionExecution),
        schemars::schema_for!(crate::functions::executions::response::unary::FunctionExecutionTask),
        schemars::schema_for!(crate::functions::executions::response::unary::Object),
        schemars::schema_for!(crate::functions::executions::response::unary::ReasoningSummary),
        schemars::schema_for!(crate::functions::executions::response::unary::Task),
        schemars::schema_for!(crate::functions::executions::response::unary::VectorCompletionTask),
        schemars::schema_for!(crate::functions::expression::AnyOfInputSchema),
        schemars::schema_for!(crate::functions::expression::ArrayInputSchema),
        schemars::schema_for!(crate::functions::expression::ArrayInputSchemaType),
        schemars::schema_for!(crate::functions::expression::AudioInputSchema),
        schemars::schema_for!(crate::functions::expression::AudioInputSchemaType),
        schemars::schema_for!(crate::functions::expression::BooleanInputSchema),
        schemars::schema_for!(crate::functions::expression::BooleanInputSchemaType),
        schemars::schema_for!(crate::functions::expression::Expression),
        schemars::schema_for!(crate::functions::expression::FileInputSchema),
        schemars::schema_for!(crate::functions::expression::FileInputSchemaType),
        schemars::schema_for!(crate::functions::expression::ImageInputSchema),
        schemars::schema_for!(crate::functions::expression::ImageInputSchemaType),
        schemars::schema_for!(crate::functions::expression::InputSchema),
        schemars::schema_for!(crate::functions::expression::InputValue),
        schemars::schema_for!(crate::functions::expression::InputValueExpression),
        schemars::schema_for!(crate::functions::expression::InputValueLog),
        schemars::schema_for!(crate::functions::expression::IntegerInputSchema),
        schemars::schema_for!(crate::functions::expression::IntegerInputSchemaType),
        schemars::schema_for!(crate::functions::expression::NumberInputSchema),
        schemars::schema_for!(crate::functions::expression::NumberInputSchemaType),
        schemars::schema_for!(crate::functions::expression::ObjectInputSchema),
        schemars::schema_for!(crate::functions::expression::ObjectInputSchemaType),
        schemars::schema_for!(crate::functions::expression::ParamsOwned),
        schemars::schema_for!(crate::functions::expression::Special),
        schemars::schema_for!(crate::functions::expression::StringInputSchema),
        schemars::schema_for!(crate::functions::expression::StringInputSchemaType),
        schemars::schema_for!(crate::functions::expression::TaskOutputOwned),
        schemars::schema_for!(crate::functions::expression::VideoInputSchema),
        schemars::schema_for!(crate::functions::expression::VideoInputSchemaType),
        schemars::schema_for!(crate::functions::inventions::DescriptionObject),
        schemars::schema_for!(crate::functions::inventions::EssayObject),
        schemars::schema_for!(crate::functions::inventions::EssayTasksObject),
        schemars::schema_for!(crate::functions::inventions::IndexObject),
        schemars::schema_for!(crate::functions::inventions::ScalarBranchTaskObject),
        schemars::schema_for!(crate::functions::inventions::ScalarInputSchemaObject),
        schemars::schema_for!(crate::functions::inventions::ScalarLeafTaskObject),
        schemars::schema_for!(crate::functions::inventions::TasksLengthObject),
        schemars::schema_for!(crate::functions::inventions::VectorBranchTaskObject),
        schemars::schema_for!(crate::functions::inventions::VectorInputSchemaObject),
        schemars::schema_for!(crate::functions::inventions::VectorLeafTaskObject),
        schemars::schema_for!(crate::functions::inventions::prompts::response::GetPromptResponse),
        schemars::schema_for!(crate::functions::inventions::prompts::InlinePrompt),
        schemars::schema_for!(crate::functions::inventions::prompts::InlinePromptOrRemoteCommitOptional),
        schemars::schema_for!(crate::functions::inventions::prompts::response::ListPromptResponse),
        schemars::schema_for!(crate::functions::inventions::prompts::request::ListPromptsRequest),
        schemars::schema_for!(crate::functions::inventions::prompts::request::ListPromptsSource),
        schemars::schema_for!(crate::functions::inventions::prompts::Prompt),
        schemars::schema_for!(crate::functions::inventions::prompts::RemotePrompt),
        schemars::schema_for!(crate::functions::inventions::prompts::StepPromptExpression),
        schemars::schema_for!(crate::functions::inventions::prompts::StepPromptType),
        schemars::schema_for!(crate::functions::inventions::prompts::response::UsagePromptResponse),
        schemars::schema_for!(crate::functions::inventions::recursive::request::FunctionInventionRecursiveCreateParams),
        schemars::schema_for!(crate::functions::inventions::recursive::request::FunctionInventionRecursiveCreateParamsLog),
        schemars::schema_for!(crate::functions::inventions::recursive::response::streaming::FunctionInventionChunk),
        schemars::schema_for!(crate::functions::inventions::recursive::response::streaming::FunctionInventionRecursiveChunk),
        schemars::schema_for!(crate::functions::inventions::recursive::response::streaming::FunctionInventionRecursiveChunkLog),
        schemars::schema_for!(crate::functions::inventions::recursive::response::streaming::InnerError<'_>),
        schemars::schema_for!(crate::functions::inventions::recursive::response::streaming::Object),
        schemars::schema_for!(crate::functions::inventions::recursive::response::unary::FunctionInvention),
        schemars::schema_for!(crate::functions::inventions::recursive::response::unary::FunctionInventionRecursive),
        schemars::schema_for!(crate::functions::inventions::recursive::response::unary::Object),
        schemars::schema_for!(crate::functions::inventions::request::FunctionInventionCreateParams),
        schemars::schema_for!(crate::functions::inventions::response::streaming::AgentCompletionChunk),
        schemars::schema_for!(crate::functions::inventions::response::streaming::FunctionInventionChunk),
        schemars::schema_for!(crate::functions::inventions::response::streaming::FunctionInventionChunkLog),
        schemars::schema_for!(crate::functions::inventions::response::streaming::InnerError<'_>),
        schemars::schema_for!(crate::functions::inventions::response::streaming::Object),
        schemars::schema_for!(crate::functions::inventions::response::unary::AgentCompletion),
        schemars::schema_for!(crate::functions::inventions::response::unary::FunctionInvention),
        schemars::schema_for!(crate::functions::inventions::response::unary::Object),
        schemars::schema_for!(crate::functions::inventions::state::AlphaScalarBranchState),
        schemars::schema_for!(crate::functions::inventions::state::AlphaScalarLeafState),
        schemars::schema_for!(crate::functions::inventions::state::AlphaScalarState),
        schemars::schema_for!(crate::functions::inventions::state::AlphaVectorBranchState),
        schemars::schema_for!(crate::functions::inventions::state::AlphaVectorLeafState),
        schemars::schema_for!(crate::functions::inventions::state::AlphaVectorState),
        schemars::schema_for!(crate::functions::inventions::state::response::GetFunctionInventionStateResponse),
        schemars::schema_for!(crate::functions::inventions::state::InputSchema),
        schemars::schema_for!(crate::functions::inventions::state::Params),
        schemars::schema_for!(crate::functions::inventions::state::ParamsState),
        schemars::schema_for!(crate::functions::inventions::state::ParamsStateOrRemoteCommitOptional),
        schemars::schema_for!(crate::functions::inventions::state::State),
        schemars::schema_for!(crate::functions::profiles::response::GetProfileResponse),
        schemars::schema_for!(crate::functions::profiles::response::ListProfileResponse),
        schemars::schema_for!(crate::functions::profiles::request::ListProfilesRequest),
        schemars::schema_for!(crate::functions::profiles::request::ListProfilesSource),
        schemars::schema_for!(crate::functions::profiles::response::UsageProfileResponse),
        schemars::schema_for!(crate::functions::profiles::computations::RetryToken),
        schemars::schema_for!(crate::functions::profiles::computations::request::DatasetItem),
        schemars::schema_for!(crate::functions::profiles::computations::request::FunctionProfileComputationCreateParams),
        schemars::schema_for!(crate::functions::profiles::computations::request::Target),
        schemars::schema_for!(crate::functions::profiles::computations::response::FittingStats),
        schemars::schema_for!(crate::functions::profiles::computations::response::streaming::FunctionExecutionChunk),
        schemars::schema_for!(crate::functions::profiles::computations::response::streaming::FunctionProfileComputationChunk),
        schemars::schema_for!(crate::functions::profiles::computations::response::streaming::Object),
        schemars::schema_for!(crate::functions::profiles::computations::response::unary::FunctionExecution),
        schemars::schema_for!(crate::functions::profiles::computations::response::unary::FunctionProfileComputation),
        schemars::schema_for!(crate::functions::profiles::computations::response::unary::Object),
        #[cfg(feature = "http")]
        #[cfg(feature = "viewer")]
        schemars::schema_for!(crate::http::viewer::AgentCompletionCreateParams),
        #[cfg(feature = "http")]
        #[cfg(feature = "viewer")]
        schemars::schema_for!(crate::http::viewer::AgentCompletionRequest),
        #[cfg(feature = "http")]
        #[cfg(feature = "viewer")]
        schemars::schema_for!(crate::http::viewer::FunctionExecutionCreateParams),
        #[cfg(feature = "http")]
        #[cfg(feature = "viewer")]
        schemars::schema_for!(crate::http::viewer::FunctionExecutionRequest),
        #[cfg(feature = "http")]
        #[cfg(feature = "viewer")]
        schemars::schema_for!(crate::http::viewer::FunctionInventionRecursiveCreateParams),
        #[cfg(feature = "http")]
        #[cfg(feature = "viewer")]
        schemars::schema_for!(crate::http::viewer::FunctionInventionRecursiveRequest),
        #[cfg(feature = "http")]
        #[cfg(feature = "viewer")]
        schemars::schema_for!(crate::http::viewer::LaboratoryExecutionCreateParams),
        #[cfg(feature = "http")]
        #[cfg(feature = "viewer")]
        schemars::schema_for!(crate::http::viewer::LaboratoryExecutionRequest),
        #[cfg(feature = "http")]
        #[cfg(feature = "viewer")]
        schemars::schema_for!(crate::http::viewer::Request),
        #[cfg(feature = "http")]
        #[cfg(feature = "viewer")]
        schemars::schema_for!(crate::http::viewer::ResponseError),
        schemars::schema_for!(crate::laboratories::executions::request::LaboratoryExecutionCreateParams),
        schemars::schema_for!(crate::laboratories::executions::response::streaming::BuilderChunk),
        schemars::schema_for!(crate::laboratories::executions::response::streaming::EvaluationChunk),
        schemars::schema_for!(crate::laboratories::executions::response::streaming::InnerError<'_>),
        schemars::schema_for!(crate::laboratories::executions::response::streaming::LaboratoryExecutionChunk),
        schemars::schema_for!(crate::laboratories::executions::response::streaming::Object),
        schemars::schema_for!(crate::laboratories::executions::response::unary::Builder),
        schemars::schema_for!(crate::laboratories::executions::response::unary::Evaluation),
        schemars::schema_for!(crate::laboratories::executions::response::unary::LaboratoryExecution),
        schemars::schema_for!(crate::laboratories::executions::response::unary::Object),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::initialize_result::CompletionsCapability),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::initialize_result::Implementation),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::initialize_result::InitializeResult),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::JsonRpcError),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::JsonRpcNotification),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::JsonRpcRequest),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::initialize_result::LoggingCapability),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::initialize_result::PromptsCapability),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::initialize_result::ResourcesCapability),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::initialize_result::ServerCapabilities),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::initialize_result::TasksCancelCapability),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::initialize_result::TasksCapability),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::initialize_result::TasksListCapability),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::initialize_result::TasksRequestsCapability),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::initialize_result::TasksToolsCallCapability),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::initialize_result::TasksToolsCapability),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::initialize_result::ToolsCapability),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::resource::ListResourcesRequest),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::resource::ListResourcesResult),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::resource::ReadResourceRequestParams),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::resource::ReadResourceResult),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::resource::Resource),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::shared::Annotations),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::shared::BlobResourceContents),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::shared::Icon),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::shared::IconTheme),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::shared::ResourceContents),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::shared::ResourceContentsUnion),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::shared::Role),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::shared::TextResourceContents),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::tool::AudioContent),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::tool::CallToolRequestParams),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::tool::CallToolResult),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::tool::ContentBlock),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::tool::EmbeddedResource),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::tool::ImageContent),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::tool::ListToolsRequest),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::tool::ListToolsResult),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::tool::ResourceLink),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::tool::TaskMetadata),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::tool::TaskSupport),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::tool::TextContent),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::tool::Tool),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::tool::ToolAnnotations),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::tool::ToolExecution),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::tool::ToolResultContent),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::tool::ToolSchemaObject),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::tool::ToolSchemaType),
        #[cfg(feature = "mcp")]
        schemars::schema_for!(crate::mcp::tool::ToolUseContent),
        schemars::schema_for!(crate::swarm::response::GetSwarmResponse),
        schemars::schema_for!(crate::swarm::InlineSwarm),
        schemars::schema_for!(crate::swarm::InlineSwarmBase),
        schemars::schema_for!(crate::swarm::InlineSwarmBaseOrRemote),
        schemars::schema_for!(crate::swarm::InlineSwarmBaseOrRemoteCommitOptional),
        schemars::schema_for!(crate::swarm::response::ListSwarmResponse),
        schemars::schema_for!(crate::swarm::request::ListSwarmsRequest),
        schemars::schema_for!(crate::swarm::request::ListSwarmsSource),
        schemars::schema_for!(crate::swarm::RemoteSwarm),
        schemars::schema_for!(crate::swarm::RemoteSwarmBase),
        schemars::schema_for!(crate::swarm::Swarm),
        schemars::schema_for!(crate::swarm::SwarmBase),
        schemars::schema_for!(crate::swarm::response::UsageSwarmResponse),
        schemars::schema_for!(crate::vector::completions::VectorResponses),
        schemars::schema_for!(crate::vector::completions::cache::response::CacheVote),
        schemars::schema_for!(crate::vector::completions::cache::request::CacheVoteRequestOwned),
        schemars::schema_for!(crate::vector::completions::cache::response::CompletionVotes),
        schemars::schema_for!(crate::vector::completions::cache::request::GetCompletionVotesRequest),
        schemars::schema_for!(crate::vector::completions::request::VectorCompletionCreateParams),
        schemars::schema_for!(crate::vector::completions::response::Vote),
        schemars::schema_for!(crate::vector::completions::response::streaming::AgentCompletionChunk),
        schemars::schema_for!(crate::vector::completions::response::streaming::InnerError<'_>),
        schemars::schema_for!(crate::vector::completions::response::streaming::Object),
        schemars::schema_for!(crate::vector::completions::response::streaming::VectorCompletionChunk),
        schemars::schema_for!(crate::vector::completions::response::streaming::VectorCompletionChunkLog),
        schemars::schema_for!(crate::vector::completions::response::unary::AgentCompletion),
        schemars::schema_for!(crate::vector::completions::response::unary::Object),
        schemars::schema_for!(crate::vector::completions::response::unary::VectorCompletion),
        #[cfg(feature = "viewer")]
        schemars::schema_for!(crate::viewer::Event),
        schemars::schema_for!(crate::auth::ApiKey),
    ];
    #[cfg(feature = "mcp")]
    {
        schemas.extend([
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::shared::Annotations),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::shared::Role),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::shared::Icon),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::shared::IconTheme),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::shared::ResourceContents),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::shared::TextResourceContents),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::shared::BlobResourceContents),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::shared::ResourceContentsUnion),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::initialize_result::InitializeResult),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::initialize_result::Implementation),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::initialize_result::ServerCapabilities),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::initialize_result::PromptsCapability),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::initialize_result::ResourcesCapability),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::initialize_result::ToolsCapability),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::initialize_result::LoggingCapability),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::initialize_result::CompletionsCapability),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::initialize_result::TasksCapability),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::initialize_result::TasksListCapability),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::initialize_result::TasksCancelCapability),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::initialize_result::TasksRequestsCapability),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::initialize_result::TasksToolsCapability),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::initialize_result::TasksToolsCallCapability),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::client_objectiveai_mcp::McpKind),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::client_objectiveai_mcp::client_request::Request),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::client_objectiveai_mcp::client_request::Payload),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::client_objectiveai_mcp::client_request::McpListChanged),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::client_objectiveai_mcp::client_request::McpListChangedKind),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::client_objectiveai_mcp::client_response::Response),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::client_objectiveai_mcp::server_request::Request),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::client_objectiveai_mcp::server_request::InitializeRequest),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::client_objectiveai_mcp::server_response::Response),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::client_objectiveai_mcp::server_response::InitializeReply),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::client_objectiveai_mcp::server_response::JsonRpcResult<crate::client_objectiveai_mcp::server_response::InitializeReply>),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::client_objectiveai_mcp::server_response::JsonRpcResult<crate::mcp::tool::ListToolsResult>),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::client_objectiveai_mcp::server_response::JsonRpcResult<crate::mcp::tool::CallToolResult>),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::client_objectiveai_mcp::server_response::JsonRpcResult<crate::mcp::resource::ListResourcesResult>),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::client_objectiveai_mcp::server_response::JsonRpcResult<crate::mcp::resource::ReadResourceResult>),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::client_objectiveai_mcp::server_response::JsonRpcResult<()>),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::JsonRpcRequest),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::JsonRpcError),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::JsonRpcNotification),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::resource::ListResourcesRequest),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::resource::ListResourcesResult),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::resource::ReadResourceRequestParams),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::resource::ReadResourceResult),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::resource::Resource),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::tool::AudioContent),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::tool::CallToolRequestParams),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::tool::CallToolResult),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::tool::TaskMetadata),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::tool::ContentBlock),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::tool::EmbeddedResource),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::tool::ImageContent),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::tool::ListToolsRequest),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::tool::ListToolsResult),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::tool::ResourceLink),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::tool::TextContent),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::tool::Tool),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::tool::ToolSchemaObject),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::tool::ToolSchemaType),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::tool::ToolAnnotations),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::tool::ToolExecution),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::tool::TaskSupport),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::tool::ToolResultContent),
            #[cfg(feature = "mcp")]
            schemars::schema_for!(crate::mcp::tool::ToolUseContent),
        ]);
    }
    #[cfg(feature = "filesystem")]
    {
        schemas.extend([
            schemars::schema_for!(crate::filesystem::config::AgentsConfig),
            schemars::schema_for!(crate::filesystem::config::ApiConfig),
            schemars::schema_for!(crate::filesystem::config::Config),
            schemars::schema_for!(crate::filesystem::config::Favorite),
            schemars::schema_for!(crate::filesystem::config::PairFavorite),
            schemars::schema_for!(crate::filesystem::config::FunctionsConfig),
            schemars::schema_for!(crate::filesystem::config::FunctionsInventionsConfig),
            schemars::schema_for!(crate::filesystem::config::FunctionsProfilesConfig),
            schemars::schema_for!(crate::filesystem::config::FunctionsProfilesPairsConfig),
            schemars::schema_for!(crate::filesystem::config::SwarmsConfig),
            schemars::schema_for!(crate::filesystem::config::ViewerSecretSignaturePair),
            schemars::schema_for!(crate::filesystem::config::ViewerConfig),
            schemars::schema_for!(crate::filesystem::config::McpConfig),
            schemars::schema_for!(crate::filesystem::logs::ListItem),
            schemars::schema_for!(crate::filesystem::logs::LogReference),
            schemars::schema_for!(crate::filesystem::logs::LogReferenceTag),
            schemars::schema_for!(crate::filesystem::logs::indexed_reference::LogReference),
            schemars::schema_for!(crate::filesystem::logs::LatestContinuation),
            // Per-site `LogReference` types (sibling to the chunk
            // that produces them; same name, distinct module paths).
            schemars::schema_for!(crate::functions::executions::response::streaming::function_execution_task_log_reference::LogReference),
            schemars::schema_for!(crate::functions::executions::response::streaming::reasoning_summary_log_reference::LogReference),
            schemars::schema_for!(crate::functions::executions::response::streaming::task_log_reference::LogReference),
            schemars::schema_for!(crate::functions::executions::response::streaming::vector_completion_task_log_reference::LogReference),
            // *Log structs (on-disk log file shapes for every
            // chunk type that has a `produce_files` implementation).
            schemars::schema_for!(crate::agent::completions::message::RichContentLog),
            schemars::schema_for!(crate::agent::completions::message::SimpleContentLog),
            schemars::schema_for!(crate::agent::completions::message::MessageLog),
            schemars::schema_for!(crate::agent::completions::message::DeveloperMessageLog),
            schemars::schema_for!(crate::agent::completions::message::SystemMessageLog),
            schemars::schema_for!(crate::agent::completions::message::UserMessageLog),
            schemars::schema_for!(crate::agent::completions::message::AssistantMessageLog),
            schemars::schema_for!(crate::agent::completions::message::ToolMessageLog),
            schemars::schema_for!(crate::agent::completions::request::AgentCompletionCreateParamsLog),
            schemars::schema_for!(crate::agent::completions::response::ToolResponseLog),
            schemars::schema_for!(crate::agent::completions::response::streaming::AgentCompletionChunkLog),
            schemars::schema_for!(crate::agent::completions::response::streaming::AssistantResponseChunkLog),
            schemars::schema_for!(crate::functions::expression::InputValueLog),
            schemars::schema_for!(crate::functions::executions::request::FunctionExecutionCreateParamsLog),
            schemars::schema_for!(crate::functions::executions::response::streaming::FunctionExecutionChunkLog),
            schemars::schema_for!(crate::functions::inventions::recursive::request::FunctionInventionRecursiveCreateParamsLog),
            schemars::schema_for!(crate::functions::inventions::recursive::response::streaming::FunctionInventionRecursiveChunkLog),
            schemars::schema_for!(crate::functions::inventions::response::streaming::FunctionInventionChunkLog),
            schemars::schema_for!(crate::vector::completions::response::streaming::VectorCompletionChunkLog),
            // Typed queue-reader schemas (Client::read_new_from_queue).
            schemars::schema_for!(crate::filesystem::logs::queue::Content),
            schemars::schema_for!(crate::filesystem::logs::queue::QueueMessage),
            schemars::schema_for!(crate::filesystem::logs::queue::QueueItem),
            schemars::schema_for!(crate::filesystem::plugins::Binaries),
            schemars::schema_for!(crate::filesystem::plugins::Manifest),
            schemars::schema_for!(crate::filesystem::plugins::ManifestWithNameAndSource),
            schemars::schema_for!(crate::filesystem::plugins::McpServer),
            schemars::schema_for!(crate::filesystem::plugins::Platform),
            schemars::schema_for!(crate::filesystem::plugins::ViewerRoute),
            schemars::schema_for!(crate::filesystem::plugins::HttpMethod),
            schemars::schema_for!(crate::filesystem::plugins::WhitelistEntry),
            schemars::schema_for!(crate::filesystem::tools::Manifest),
            schemars::schema_for!(crate::filesystem::tools::ManifestWithNameAndSource),
        ]);
    }
    #[cfg(feature = "cli")]
    {
        schemas.extend([
            #[cfg(feature = "cli")]
            schemars::schema_for!(crate::cli::Error),
            #[cfg(feature = "cli")]
            schemars::schema_for!(crate::cli::ErrorType),
            #[cfg(feature = "cli")]
            schemars::schema_for!(crate::cli::Level),
            #[cfg(feature = "cli")]
            schemars::schema_for!(crate::cli::plugins::Command),
            #[cfg(feature = "cli")]
            schemars::schema_for!(crate::cli::plugins::CommandType),
            #[cfg(feature = "cli")]
            schemars::schema_for!(crate::cli::plugins::Output),
            #[cfg(feature = "cli")]
            schemars::schema_for!(crate::cli::command::plugins::run::Mcp),
            #[cfg(feature = "cli")]
            schemars::schema_for!(crate::cli::command::plugins::run::McpType),
        ]);
    }
    #[cfg(feature = "viewer")]
    {
        schemas.extend([schemars::schema_for!(crate::viewer::Event)]);
    }
    #[cfg(feature = "http")]
    {
        schemas.extend([
            #[cfg(feature = "http")]
            schemars::schema_for!(crate::http::viewer::ResponseError),
            schemars::schema_for!(
                crate::http::viewer::AgentCompletionCreateParams
            ),
            #[cfg(feature = "http")]
            schemars::schema_for!(crate::http::viewer::AgentCompletionRequest),
            schemars::schema_for!(
                crate::http::viewer::FunctionExecutionCreateParams
            ),
            schemars::schema_for!(
                crate::http::viewer::FunctionExecutionRequest
            ),
            schemars::schema_for!(
                crate::http::viewer::FunctionInventionRecursiveCreateParams
            ),
            schemars::schema_for!(
                crate::http::viewer::FunctionInventionRecursiveRequest
            ),
            schemars::schema_for!(
                crate::http::viewer::LaboratoryExecutionCreateParams
            ),
            schemars::schema_for!(
                crate::http::viewer::LaboratoryExecutionRequest
            ),
            #[cfg(feature = "http")]
            schemars::schema_for!(crate::http::viewer::Request),
        ]);
    }

    // Concrete instantiations of `WithExpression<T>`, referenced by
    // other schemas via `$ref` but whose titles only exist after
    // concrete substitution. (`serde_json::Value` is NOT registered:
    // it always inlines as an unconstrained `{}` schema wherever it
    // appears — fields and the cli `ResponseSchema` wrapper alike —
    // so no standalone "AnyValue" title exists.)
    schemas.extend([
        schemars::schema_for!(
            crate::functions::expression::WithExpression<String>
        ),
        schemars::schema_for!(
            crate::functions::expression::WithExpression<
                crate::functions::expression::InputValueExpression,
            >
        ),
        schemars::schema_for!(
            crate::functions::expression::WithExpression<
                crate::agent::completions::message::AssistantToolCallExpression,
            >
        ),
        schemars::schema_for!(
            crate::functions::expression::WithExpression<
                crate::agent::completions::message::AssistantToolCallFunctionExpression,
            >
        ),
        schemars::schema_for!(
            crate::functions::expression::WithExpression<
                crate::agent::completions::message::File,
            >
        ),
        schemars::schema_for!(
            crate::functions::expression::WithExpression<
                crate::agent::completions::message::ImageUrl,
            >
        ),
        schemars::schema_for!(
            crate::functions::expression::WithExpression<
                crate::agent::completions::message::InputAudio,
            >
        ),
        schemars::schema_for!(
            crate::functions::expression::WithExpression<
                crate::agent::completions::message::MessageExpression,
            >
        ),
        schemars::schema_for!(
            crate::functions::expression::WithExpression<
                crate::agent::completions::message::RichContentExpression,
            >
        ),
        schemars::schema_for!(
            crate::functions::expression::WithExpression<
                crate::agent::completions::message::RichContentPartExpression,
            >
        ),
        schemars::schema_for!(
            crate::functions::expression::WithExpression<
                crate::agent::completions::message::SimpleContentExpression,
            >
        ),
        schemars::schema_for!(
            crate::functions::expression::WithExpression<
                crate::agent::completions::message::SimpleContentPartExpression,
            >
        ),
        schemars::schema_for!(
            crate::functions::expression::WithExpression<
                crate::agent::completions::message::VideoUrl,
            >
        ),
        schemars::schema_for!(
            crate::functions::expression::WithExpression<
                Vec<
                    crate::functions::expression::WithExpression<
                        crate::agent::completions::message::AssistantToolCallExpression,
                    >,
                >,
            >
        ),
        schemars::schema_for!(
            crate::functions::expression::WithExpression<
                Vec<
                    crate::functions::expression::WithExpression<
                        crate::agent::completions::message::MessageExpression,
                    >,
                >,
            >
        ),
        schemars::schema_for!(
            crate::functions::expression::WithExpression<
                Vec<
                    crate::functions::expression::WithExpression<
                        crate::agent::completions::message::RichContentExpression,
                    >,
                >,
            >
        ),
    ]);

    schemas
}

/// Schema helper: produces a `$ref` to `T` for use with `#[schemars(schema_with)]`
/// on `#[serde(flatten)]` fields, preventing schemars from merging the inner type's
/// schema into the parent.
pub fn flatten_schema<T: schemars::JsonSchema>(
    generator: &mut schemars::SchemaGenerator,
) -> schemars::Schema {
    generator.subschema_for::<T>()
}
