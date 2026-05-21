pub trait UpstreamError:
    std::error::Error + objectiveai_sdk::error::StatusError + Send + Sync + 'static
{
}

impl<T> UpstreamError for T where
    T: std::error::Error + objectiveai_sdk::error::StatusError + Send + Sync + 'static
{
}

/// The first stream item must never be an error chunk. If the upstream
/// would fail before producing any non-error chunk, it must return
/// `Err(...)` from `create` instead of yielding an error chunk into
/// the stream.
///
/// The stream must never be empty. If the upstream produces no chunks
/// at all, it must return `Err(...)` from `create` instead of an
/// empty stream.
pub trait UpstreamClient<AGENT, CONTINUATION> {
    type State: Send + Sync + 'static;
    type Stream: futures::Stream<Item = StreamItem<Self::State>>
        + Send
        + 'static;
    type Error: UpstreamError;
    fn create(
        &self,
        // unique identifier for this completion
        id: &str,
        // unix timestamp when the completion was created
        created: u64,
        // the agent that the upstream client uses
        agent: &AGENT,
        // optional continuation from the public API request
        request_continuation: Option<&CONTINUATION>,
        // the original request params for the agent completion
        params: &objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams,
        // contains the full prompt, including from the params and from the agent
        // upstream clients do not handle merging params and agent messages
        messages: &[objectiveai_sdk::agent::completions::message::Message],
        // the single MCP connection for this agent — already initialized
        // against the in-process mcp-proxy with `X-MCP-Servers` listing
        // the agent's declared upstreams (and the invention server URL,
        // when applicable). `None` means the agent has no MCP work to do.
        // The upstream is responsible for sourcing its tool list from
        // this connection (e.g. via `list_tools`); the orchestrator no
        // longer pre-resolves tool names or maps for the upstream.
        mcp_connection: Option<objectiveai_sdk::mcp::Connection>,
        // a continuation from a previous agent completion
        // the upstream client can continue conversations from previous state
        // the agent may change
        continuation: Option<&[super::ContinuationItem<Self::State>]>,
        // optional user-provided API key (BYOK) — used as authorization if provided
        byok: Option<&str>,
        // cost multiplier for usage reporting
        cost_multiplier: rust_decimal::Decimal,
        // when false, the model should not be allowed to call tools
        tools_enabled: bool,
        // invention context — only set when called from the invention client
        invention_type: Option<objectiveai_sdk::functions::inventions::prompts::StepPromptType>,
        invention_step: Option<usize>,
        invention_tasks_min: Option<u64>,
        invention_input_schema: Option<String>,
        // composite agent id `{parent}/{agent.id}_{index}` (or
        // `{agent.id}_{index}` with no parent) that the orchestrator
        // already sends to the MCP proxy. Runner-backed upstreams
        // forward this as `OBJECTIVEAI_AGENT_ID` in the env they hand
        // to their child SDK subprocess; openrouter and mock ignore it.
        agent_id_header: Option<&str>,
    ) -> impl Future<
        Output = Result<
            Self::Stream,
            Self::Error,
        >,
    > + Send
    + 'static;

    /// Builds a response continuation from the proxy session info
    /// (proxy URL → agent's session id, max one entry), the request
    /// continuation, the messages, and internal continuation items.
    fn response_continuation(
        &self,
        mcp_sessions: indexmap::IndexMap<String, String>,
        request_continuation: Option<&CONTINUATION>,
        messages: &[objectiveai_sdk::agent::completions::message::Message],
        continuation: Option<&[super::ContinuationItem<Self::State>]>,
    ) -> CONTINUATION;
}

pub struct UnimplementedUpstreamClient;

impl<AGENT, CONTINUATION> UpstreamClient<AGENT, CONTINUATION> for UnimplementedUpstreamClient {
    type State = ();
    type Stream = futures::stream::Empty<StreamItem<Self::State>>;
    type Error = objectiveai_sdk::error::ResponseError;
    fn create(
        &self,
        _id: &str,
        _created: u64,
        _agent: &AGENT,
        _request_continuation: Option<&CONTINUATION>,
        _params: &objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams,
        _messages: &[objectiveai_sdk::agent::completions::message::Message],
        _mcp_connection: Option<objectiveai_sdk::mcp::Connection>,
        _continuation: Option<&[super::ContinuationItem<Self::State>]>,
        _byok: Option<&str>,
        _cost_multiplier: rust_decimal::Decimal,
        _tools_enabled: bool,
        _invention_type: Option<objectiveai_sdk::functions::inventions::prompts::StepPromptType>,
        _invention_step: Option<usize>,
        _invention_tasks_min: Option<u64>,
        _invention_input_schema: Option<String>,
        _agent_id_header: Option<&str>,
    ) -> impl Future<
        Output = Result<
            Self::Stream,
            Self::Error,
        >,
    > + Send
    + 'static {
        async {
            Err(
                objectiveai_sdk::error::ResponseError {
                    code: 501,
                    message: serde_json::Value::Null,
                }
            )
        }
    }

    fn response_continuation(
        &self,
        _mcp_sessions: indexmap::IndexMap<String, String>,
        _request_continuation: Option<&CONTINUATION>,
        _messages: &[objectiveai_sdk::agent::completions::message::Message],
        _continuation: Option<&[super::ContinuationItem<Self::State>]>,
    ) -> CONTINUATION {
        unimplemented!()
    }
}

#[derive(Debug, Clone)]
pub enum StreamItem<STATE> {
    Chunk(objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk),
    State(STATE),
}
