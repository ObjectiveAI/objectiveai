/// Returns JSON Schemas for all public types that derive `JsonSchema`.
///
/// Each schema is generated via `schemars::schema_for!()` and includes
/// the type's `$defs` for referenced types. The schema title matches
/// the `#[schemars(rename = "...")]` attribute on each type.
pub fn json_schemas() -> Vec<schemars::Schema> {
    let mut schemas = vec![
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
        schemars::schema_for!(crate::agent::completions::message::AssistantToolCall),
        schemars::schema_for!(crate::agent::completions::message::AssistantToolCallDelta),
        schemars::schema_for!(crate::agent::completions::message::AssistantToolCallExpression),
        schemars::schema_for!(crate::agent::completions::message::AssistantToolCallFunction),
        schemars::schema_for!(crate::agent::completions::message::AssistantToolCallFunctionDelta),
        schemars::schema_for!(crate::agent::completions::message::AssistantToolCallFunctionExpression),
        schemars::schema_for!(crate::agent::completions::message::AssistantToolCallType),
        schemars::schema_for!(crate::agent::completions::message::DeveloperMessage),
        schemars::schema_for!(crate::agent::completions::message::DeveloperMessageExpression),
        schemars::schema_for!(crate::agent::completions::message::File),
        schemars::schema_for!(crate::agent::completions::message::ImageUrl),
        schemars::schema_for!(crate::agent::completions::message::ImageUrlDetail),
        schemars::schema_for!(crate::agent::completions::message::InputAudio),
        schemars::schema_for!(crate::agent::completions::message::Message),
        schemars::schema_for!(crate::agent::completions::message::MessageExpression),
        schemars::schema_for!(crate::agent::completions::message::PipeAck),
        schemars::schema_for!(crate::agent::completions::message::RichContent),
        schemars::schema_for!(crate::agent::completions::message::RichContentExpression),
        schemars::schema_for!(crate::agent::completions::message::RichContentPart),
        schemars::schema_for!(crate::agent::completions::message::RichContentPartExpression),
        schemars::schema_for!(crate::agent::completions::message::SimpleContent),
        schemars::schema_for!(crate::agent::completions::message::SimpleContentExpression),
        schemars::schema_for!(crate::agent::completions::message::SimpleContentPart),
        schemars::schema_for!(crate::agent::completions::message::SimpleContentPartExpression),
        schemars::schema_for!(crate::agent::completions::message::SystemMessage),
        schemars::schema_for!(crate::agent::completions::message::SystemMessageExpression),
        schemars::schema_for!(crate::agent::completions::message::ToolMessage),
        schemars::schema_for!(crate::agent::completions::message::ToolMessageExpression),
        schemars::schema_for!(crate::agent::completions::message::ToolResponseMetadata),
        schemars::schema_for!(crate::agent::completions::message::UserMessage),
        schemars::schema_for!(crate::agent::completions::message::UserMessageExpression),
        schemars::schema_for!(crate::agent::completions::message::VideoUrl),
        schemars::schema_for!(crate::agent::completions::request::AgentCompletionCreateParams),
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
        schemars::schema_for!(crate::agent::completions::response::ToolRole),
        schemars::schema_for!(crate::agent::completions::response::TopLogprob),
        schemars::schema_for!(crate::agent::completions::response::UpstreamUsage),
        schemars::schema_for!(crate::agent::completions::response::Usage),
        schemars::schema_for!(crate::agent::completions::response::streaming::AgentCompletionChunk),
        schemars::schema_for!(crate::agent::completions::response::streaming::AssistantResponseChunk),
        schemars::schema_for!(crate::agent::completions::response::streaming::MessageChunk),
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
        schemars::schema_for!(crate::cli::command::agents::Request),
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
        schemars::schema_for!(crate::cli::command::agents::instances::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::list::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::list::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::list::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::list::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::list::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::list::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::list::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::list::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::list::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::list::RequestSource),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::list::ResponseFavorite),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::list::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::list::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::list::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::list::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::list::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::me::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::me::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::me::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::me::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::me::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::me::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::me::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::message::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::message::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::message::RequestMessage),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::message::RequestDangerousAdvanced),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::message::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::message::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::message::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::message::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::message::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::message::response_schema::Request),
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
        schemars::schema_for!(crate::cli::command::agents::instances::read::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::read::all::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::read::all::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::read::all::ResponseContent),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::read::all::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::read::all::ResponseQueueItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::read::all::ResponseQueueMessage),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::read::all::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::read::all::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::read::all::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::read::all::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::read::id::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::read::id::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::read::id::Response),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::read::id::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::read::id::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::read::id::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::read::id::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::read::pending::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::read::pending::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::read::pending::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::read::pending::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::read::pending::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::read::pending::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::read::pending::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::read::subscribe::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::read::subscribe::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::read::subscribe::RequestMessageKind),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::read::subscribe::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::read::subscribe::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::read::subscribe::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::read::subscribe::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::read::subscribe::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::spawn::AgentResolution),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::spawn::AgentSpec),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::spawn::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::spawn::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::spawn::RequestDangerousAdvanced),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::spawn::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::spawn::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::spawn::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::spawn::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::agents::instances::spawn::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::agents::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::agents::favorites::Request),
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
        schemars::schema_for!(crate::cli::command::config::functions::favorites::Request),
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
        schemars::schema_for!(crate::cli::command::config::functions::profiles::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::functions::profiles::favorites::Request),
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
        schemars::schema_for!(crate::cli::command::config::functions::profiles::pairs::favorites::Request),
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
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::mcp::address::Request),
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
        schemars::schema_for!(crate::cli::command::config::swarms::favorites::Request),
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
        #[cfg(feature = "viewer")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::config::viewer::address::Request),
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
        schemars::schema_for!(crate::cli::command::functions::execute::FunctionSpec),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::execute::ProfileSpec),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::execute::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::execute::standard::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::execute::standard::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::execute::standard::RequestDangerousAdvanced),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::execute::standard::RequestInput),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::execute::standard::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::execute::standard::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::execute::standard::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::execute::standard::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::execute::standard::response_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::execute::swiss_system::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::execute::swiss_system::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::execute::swiss_system::RequestDangerousAdvanced),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::execute::swiss_system::RequestInput),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::execute::swiss_system::ResponseItem),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::execute::swiss_system::request_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::execute::swiss_system::request_schema::Request),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::execute::swiss_system::response_schema::Path),
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::functions::execute::swiss_system::response_schema::Request),
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
        #[cfg(feature = "mcp")]
        #[cfg(feature = "cli")]
        schemars::schema_for!(crate::cli::command::mcp::Request),
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
        schemars::schema_for!(crate::functions::executions::request::Reasoning),
        schemars::schema_for!(crate::functions::executions::request::Strategy),
        schemars::schema_for!(crate::functions::executions::response::Output),
        schemars::schema_for!(crate::functions::executions::response::streaming::FunctionExecutionChunk),
        schemars::schema_for!(crate::functions::executions::response::streaming::FunctionExecutionTaskChunk),
        schemars::schema_for!(crate::functions::executions::response::streaming::InnerError<'_>),
        schemars::schema_for!(crate::functions::executions::response::streaming::Object),
        schemars::schema_for!(crate::functions::executions::response::streaming::ReasoningSummaryChunk),
        schemars::schema_for!(crate::functions::executions::response::streaming::TaskChunk),
        schemars::schema_for!(crate::functions::executions::response::streaming::VectorCompletionTaskChunk),
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
        schemars::schema_for!(crate::functions::inventions::recursive::response::streaming::FunctionInventionChunk),
        schemars::schema_for!(crate::functions::inventions::recursive::response::streaming::FunctionInventionRecursiveChunk),
        schemars::schema_for!(crate::functions::inventions::recursive::response::streaming::InnerError<'_>),
        schemars::schema_for!(crate::functions::inventions::recursive::response::streaming::Object),
        schemars::schema_for!(crate::functions::inventions::recursive::response::unary::FunctionInvention),
        schemars::schema_for!(crate::functions::inventions::recursive::response::unary::FunctionInventionRecursive),
        schemars::schema_for!(crate::functions::inventions::recursive::response::unary::Object),
        schemars::schema_for!(crate::functions::inventions::request::FunctionInventionCreateParams),
        schemars::schema_for!(crate::functions::inventions::response::streaming::AgentCompletionChunk),
        schemars::schema_for!(crate::functions::inventions::response::streaming::FunctionInventionChunk),
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
        schemars::schema_for!(crate::vector::completions::response::unary::AgentCompletion),
        schemars::schema_for!(crate::vector::completions::response::unary::Object),
        schemars::schema_for!(crate::vector::completions::response::unary::VectorCompletion),
        #[cfg(feature = "viewer")]
        schemars::schema_for!(crate::viewer::Event),
        schemars::schema_for!(crate::auth::ApiKey),
    ];
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
