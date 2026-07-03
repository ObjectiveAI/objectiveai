//! The [`Run`] envelope enum + wire dispatch for
//! [`super::WebSocketListener`] — one variant per command leaf,
//! discriminated over the run's REQUEST (`path_type`, plus the
//! `dangerous_advanced.stream` flag on the dual-form leaves). Each
//! variant carries exactly three things: the actual leaf request, the
//! producer's [`AgentArguments`], and the response — a
//! [`UnaryResponse`] future for unary leaves, a
//! [`ResponseItemStream`] for streaming leaves. One fallback:
//! [`Run::Transformed`], for requests carrying a jq/python output
//! transform (their items are post-transform JSON, kept as raw
//! text). A run whose `path_type` this build's types predate — or
//! whose request fails to deserialize — is SKIPPED: no envelope, its
//! frames dropped.
//!
//! Bodies arrive as [`RawValue`] wire text and deserialization is
//! deferred until the dispatch knows exactly what type each one is —
//! requests and items parse straight from the wire, never through a
//! `serde_json::Value` intermediate (which would route numbers
//! through `f64` and clobber high-precision decimals).
//!
//! Deliberately NOT serde-serializable — this is an in-process
//! consumption surface, not a wire type; nothing here touches the
//! json-schema set.
//!
//! Mechanically authored from the command tree (every scope with a
//! `Path` discriminator, classified unary/streaming/dual-form by its
//! `execute`/`execute_streaming` returns — the same discovery the
//! JS codegen uses). A leaf added later and missed here is skipped
//! at runtime; extend the enum when touched.

use serde_json::value::RawValue;

use crate::cli::command::AgentArguments;

use super::listener::{ResponseItemStream, UnaryResponse, decode_item};

/// One daemon-broadcast run, typed off its request. See the module
/// docs.
#[allow(clippy::large_enum_variant)]
pub enum Run {
    /// `agents enqueue` — unary.
    AgentsEnqueue {
        request: crate::cli::command::agents::enqueue::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::enqueue::Response>,
    },
    /// `agents enqueue request_schema` — unary.
    AgentsEnqueueRequestSchema {
        request: crate::cli::command::agents::enqueue::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::enqueue::request_schema::Response>,
    },
    /// `agents enqueue response_schema` — unary.
    AgentsEnqueueResponseSchema {
        request: crate::cli::command::agents::enqueue::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::enqueue::response_schema::Response>,
    },
    /// `agents get` — unary.
    AgentsGet {
        request: crate::cli::command::agents::get::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::get::Response>,
    },
    /// `agents get request_schema` — unary.
    AgentsGetRequestSchema {
        request: crate::cli::command::agents::get::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::get::request_schema::Response>,
    },
    /// `agents get response_schema` — unary.
    AgentsGetResponseSchema {
        request: crate::cli::command::agents::get::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::get::response_schema::Response>,
    },
    /// `agents instances get` — streaming.
    AgentsInstancesGet {
        request: crate::cli::command::agents::instances::get::Request,
        agent_arguments: AgentArguments,
        response: ResponseItemStream<crate::cli::command::agents::instances::get::ResponseItem>,
    },
    /// `agents instances get request_schema` — unary.
    AgentsInstancesGetRequestSchema {
        request: crate::cli::command::agents::instances::get::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::instances::get::request_schema::Response>,
    },
    /// `agents instances get response_schema` — unary.
    AgentsInstancesGetResponseSchema {
        request: crate::cli::command::agents::instances::get::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::instances::get::response_schema::Response>,
    },
    /// `agents instances list` — streaming.
    AgentsInstancesList {
        request: crate::cli::command::agents::instances::list::Request,
        agent_arguments: AgentArguments,
        response: ResponseItemStream<crate::cli::command::agents::instances::list::ResponseItem>,
    },
    /// `agents instances list request_schema` — unary.
    AgentsInstancesListRequestSchema {
        request: crate::cli::command::agents::instances::list::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::instances::list::request_schema::Response>,
    },
    /// `agents instances list response_schema` — unary.
    AgentsInstancesListResponseSchema {
        request: crate::cli::command::agents::instances::list::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::instances::list::response_schema::Response>,
    },
    /// `agents laboratories attach` — unary.
    AgentsLaboratoriesAttach {
        request: crate::cli::command::agents::laboratories::attach::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::laboratories::attach::Response>,
    },
    /// `agents laboratories attach request_schema` — unary.
    AgentsLaboratoriesAttachRequestSchema {
        request: crate::cli::command::agents::laboratories::attach::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::laboratories::attach::request_schema::Response>,
    },
    /// `agents laboratories attach response_schema` — unary.
    AgentsLaboratoriesAttachResponseSchema {
        request: crate::cli::command::agents::laboratories::attach::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::laboratories::attach::response_schema::Response>,
    },
    /// `agents laboratories detach` — unary.
    AgentsLaboratoriesDetach {
        request: crate::cli::command::agents::laboratories::detach::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::laboratories::detach::Response>,
    },
    /// `agents laboratories detach request_schema` — unary.
    AgentsLaboratoriesDetachRequestSchema {
        request: crate::cli::command::agents::laboratories::detach::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::laboratories::detach::request_schema::Response>,
    },
    /// `agents laboratories detach response_schema` — unary.
    AgentsLaboratoriesDetachResponseSchema {
        request: crate::cli::command::agents::laboratories::detach::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::laboratories::detach::response_schema::Response>,
    },
    /// `agents laboratories list` — streaming.
    AgentsLaboratoriesList {
        request: crate::cli::command::agents::laboratories::list::Request,
        agent_arguments: AgentArguments,
        response: ResponseItemStream<crate::cli::command::agents::laboratories::list::ResponseItem>,
    },
    /// `agents laboratories list request_schema` — unary.
    AgentsLaboratoriesListRequestSchema {
        request: crate::cli::command::agents::laboratories::list::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::laboratories::list::request_schema::Response>,
    },
    /// `agents laboratories list response_schema` — unary.
    AgentsLaboratoriesListResponseSchema {
        request: crate::cli::command::agents::laboratories::list::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::laboratories::list::response_schema::Response>,
    },
    /// `agents list` — streaming.
    AgentsList {
        request: crate::cli::command::agents::list::Request,
        agent_arguments: AgentArguments,
        response: ResponseItemStream<crate::cli::command::agents::list::ResponseItem>,
    },
    /// `agents list request_schema` — unary.
    AgentsListRequestSchema {
        request: crate::cli::command::agents::list::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::list::request_schema::Response>,
    },
    /// `agents list response_schema` — unary.
    AgentsListResponseSchema {
        request: crate::cli::command::agents::list::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::list::response_schema::Response>,
    },
    /// `agents logs list` — streaming.
    AgentsLogsList {
        request: crate::cli::command::agents::logs::list::Request,
        agent_arguments: AgentArguments,
        response: ResponseItemStream<crate::cli::command::agents::logs::list::ResponseItem>,
    },
    /// `agents logs list request_schema` — unary.
    AgentsLogsListRequestSchema {
        request: crate::cli::command::agents::logs::list::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::logs::list::request_schema::Response>,
    },
    /// `agents logs list response_schema` — unary.
    AgentsLogsListResponseSchema {
        request: crate::cli::command::agents::logs::list::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::logs::list::response_schema::Response>,
    },
    /// `agents logs open` — unary.
    AgentsLogsOpen {
        request: crate::cli::command::agents::logs::open::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::logs::open::Response>,
    },
    /// `agents logs open request_schema` — unary.
    AgentsLogsOpenRequestSchema {
        request: crate::cli::command::agents::logs::open::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::logs::open::request_schema::Response>,
    },
    /// `agents logs open response_schema` — unary.
    AgentsLogsOpenResponseSchema {
        request: crate::cli::command::agents::logs::open::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::logs::open::response_schema::Response>,
    },
    /// `agents logs subscribe` — streaming.
    AgentsLogsSubscribe {
        request: crate::cli::command::agents::logs::subscribe::Request,
        agent_arguments: AgentArguments,
        response: ResponseItemStream<crate::cli::command::agents::logs::subscribe::ResponseItem>,
    },
    /// `agents logs subscribe request_schema` — unary.
    AgentsLogsSubscribeRequestSchema {
        request: crate::cli::command::agents::logs::subscribe::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::logs::subscribe::request_schema::Response>,
    },
    /// `agents logs subscribe response_schema` — unary.
    AgentsLogsSubscribeResponseSchema {
        request: crate::cli::command::agents::logs::subscribe::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::logs::subscribe::response_schema::Response>,
    },
    /// `agents logs token_usage get` — unary.
    AgentsLogsTokenUsageGet {
        request: crate::cli::command::agents::logs::token_usage::get::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::logs::token_usage::get::Response>,
    },
    /// `agents logs token_usage get request_schema` — unary.
    AgentsLogsTokenUsageGetRequestSchema {
        request: crate::cli::command::agents::logs::token_usage::get::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::logs::token_usage::get::request_schema::Response>,
    },
    /// `agents logs token_usage get response_schema` — unary.
    AgentsLogsTokenUsageGetResponseSchema {
        request: crate::cli::command::agents::logs::token_usage::get::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::logs::token_usage::get::response_schema::Response>,
    },
    /// `agents logs token_usage subscribe` — streaming.
    AgentsLogsTokenUsageSubscribe {
        request: crate::cli::command::agents::logs::token_usage::subscribe::Request,
        agent_arguments: AgentArguments,
        response: ResponseItemStream<crate::cli::command::agents::logs::token_usage::subscribe::ResponseItem>,
    },
    /// `agents logs token_usage subscribe request_schema` — unary.
    AgentsLogsTokenUsageSubscribeRequestSchema {
        request: crate::cli::command::agents::logs::token_usage::subscribe::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::logs::token_usage::subscribe::request_schema::Response>,
    },
    /// `agents logs token_usage subscribe response_schema` — unary.
    AgentsLogsTokenUsageSubscribeResponseSchema {
        request: crate::cli::command::agents::logs::token_usage::subscribe::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::logs::token_usage::subscribe::response_schema::Response>,
    },
    /// `agents mcp resources list` — unary.
    AgentsMcpResourcesList {
        request: crate::cli::command::agents::mcp::resources::list::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::mcp::resources::list::Response>,
    },
    /// `agents mcp resources list request_schema` — unary.
    AgentsMcpResourcesListRequestSchema {
        request: crate::cli::command::agents::mcp::resources::list::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::mcp::resources::list::request_schema::Response>,
    },
    /// `agents mcp resources list response_schema` — unary.
    AgentsMcpResourcesListResponseSchema {
        request: crate::cli::command::agents::mcp::resources::list::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::mcp::resources::list::response_schema::Response>,
    },
    /// `agents mcp resources read` — unary.
    AgentsMcpResourcesRead {
        request: crate::cli::command::agents::mcp::resources::read::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::mcp::resources::read::Response>,
    },
    /// `agents mcp resources read request_schema` — unary.
    AgentsMcpResourcesReadRequestSchema {
        request: crate::cli::command::agents::mcp::resources::read::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::mcp::resources::read::request_schema::Response>,
    },
    /// `agents mcp resources read response_schema` — unary.
    AgentsMcpResourcesReadResponseSchema {
        request: crate::cli::command::agents::mcp::resources::read::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::mcp::resources::read::response_schema::Response>,
    },
    /// `agents mcp servers list` — unary.
    AgentsMcpServersList {
        request: crate::cli::command::agents::mcp::servers::list::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::mcp::servers::list::Response>,
    },
    /// `agents mcp servers list request_schema` — unary.
    AgentsMcpServersListRequestSchema {
        request: crate::cli::command::agents::mcp::servers::list::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::mcp::servers::list::request_schema::Response>,
    },
    /// `agents mcp servers list response_schema` — unary.
    AgentsMcpServersListResponseSchema {
        request: crate::cli::command::agents::mcp::servers::list::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::mcp::servers::list::response_schema::Response>,
    },
    /// `agents mcp tools call` — unary.
    AgentsMcpToolsCall {
        request: crate::cli::command::agents::mcp::tools::call::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::mcp::tools::call::Response>,
    },
    /// `agents mcp tools call request_schema` — unary.
    AgentsMcpToolsCallRequestSchema {
        request: crate::cli::command::agents::mcp::tools::call::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::mcp::tools::call::request_schema::Response>,
    },
    /// `agents mcp tools call response_schema` — unary.
    AgentsMcpToolsCallResponseSchema {
        request: crate::cli::command::agents::mcp::tools::call::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::mcp::tools::call::response_schema::Response>,
    },
    /// `agents mcp tools list` — unary.
    AgentsMcpToolsList {
        request: crate::cli::command::agents::mcp::tools::list::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::mcp::tools::list::Response>,
    },
    /// `agents mcp tools list request_schema` — unary.
    AgentsMcpToolsListRequestSchema {
        request: crate::cli::command::agents::mcp::tools::list::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::mcp::tools::list::request_schema::Response>,
    },
    /// `agents mcp tools list response_schema` — unary.
    AgentsMcpToolsListResponseSchema {
        request: crate::cli::command::agents::mcp::tools::list::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::mcp::tools::list::response_schema::Response>,
    },
    /// `agents message` — unary.
    AgentsMessage {
        request: crate::cli::command::agents::message::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::message::Response>,
    },
    /// `agents message request_schema` — unary.
    AgentsMessageRequestSchema {
        request: crate::cli::command::agents::message::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::message::request_schema::Response>,
    },
    /// `agents message response_schema` — unary.
    AgentsMessageResponseSchema {
        request: crate::cli::command::agents::message::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::message::response_schema::Response>,
    },
    /// `agents publish` — unary.
    AgentsPublish {
        request: crate::cli::command::agents::publish::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::publish::Response>,
    },
    /// `agents publish request_schema` — unary.
    AgentsPublishRequestSchema {
        request: crate::cli::command::agents::publish::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::publish::request_schema::Response>,
    },
    /// `agents publish response_schema` — unary.
    AgentsPublishResponseSchema {
        request: crate::cli::command::agents::publish::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::publish::response_schema::Response>,
    },
    /// `agents queue delete` — unary.
    AgentsQueueDelete {
        request: crate::cli::command::agents::queue::delete::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::queue::delete::Response>,
    },
    /// `agents queue delete request_schema` — unary.
    AgentsQueueDeleteRequestSchema {
        request: crate::cli::command::agents::queue::delete::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::queue::delete::request_schema::Response>,
    },
    /// `agents queue delete response_schema` — unary.
    AgentsQueueDeleteResponseSchema {
        request: crate::cli::command::agents::queue::delete::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::queue::delete::response_schema::Response>,
    },
    /// `agents queue deliver` — streaming.
    AgentsQueueDeliver {
        request: crate::cli::command::agents::queue::deliver::Request,
        agent_arguments: AgentArguments,
        response: ResponseItemStream<crate::cli::command::agents::queue::deliver::ResponseItem>,
    },
    /// `agents queue deliver request_schema` — unary.
    AgentsQueueDeliverRequestSchema {
        request: crate::cli::command::agents::queue::deliver::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::queue::deliver::request_schema::Response>,
    },
    /// `agents queue deliver response_schema` — unary.
    AgentsQueueDeliverResponseSchema {
        request: crate::cli::command::agents::queue::deliver::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::queue::deliver::response_schema::Response>,
    },
    /// `agents queue list` — streaming.
    AgentsQueueList {
        request: crate::cli::command::agents::queue::list::Request,
        agent_arguments: AgentArguments,
        response: ResponseItemStream<crate::cli::command::agents::queue::list::ResponseItem>,
    },
    /// `agents queue list request_schema` — unary.
    AgentsQueueListRequestSchema {
        request: crate::cli::command::agents::queue::list::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::queue::list::request_schema::Response>,
    },
    /// `agents queue list response_schema` — unary.
    AgentsQueueListResponseSchema {
        request: crate::cli::command::agents::queue::list::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::queue::list::response_schema::Response>,
    },
    /// `agents queue open` — unary.
    AgentsQueueOpen {
        request: crate::cli::command::agents::queue::open::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::queue::open::Response>,
    },
    /// `agents queue open request_schema` — unary.
    AgentsQueueOpenRequestSchema {
        request: crate::cli::command::agents::queue::open::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::queue::open::request_schema::Response>,
    },
    /// `agents queue open response_schema` — unary.
    AgentsQueueOpenResponseSchema {
        request: crate::cli::command::agents::queue::open::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::queue::open::response_schema::Response>,
    },
    /// `agents spawn` — unary form.
    AgentsSpawn {
        request: crate::cli::command::agents::spawn::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::spawn::Response>,
    },
    /// `agents spawn` — streaming form (`dangerous_advanced.stream: true`).
    AgentsSpawnStreaming {
        request: crate::cli::command::agents::spawn::Request,
        agent_arguments: AgentArguments,
        response: ResponseItemStream<crate::cli::command::agents::spawn::ResponseItem>,
    },
    /// `agents spawn request_schema` — unary.
    AgentsSpawnRequestSchema {
        request: crate::cli::command::agents::spawn::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::spawn::request_schema::Response>,
    },
    /// `agents spawn response_schema` — unary.
    AgentsSpawnResponseSchema {
        request: crate::cli::command::agents::spawn::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::spawn::response_schema::Response>,
    },
    /// `agents tags apply` — unary.
    AgentsTagsApply {
        request: crate::cli::command::agents::tags::apply::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::tags::apply::Response>,
    },
    /// `agents tags apply request_schema` — unary.
    AgentsTagsApplyRequestSchema {
        request: crate::cli::command::agents::tags::apply::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::tags::apply::request_schema::Response>,
    },
    /// `agents tags apply response_schema` — unary.
    AgentsTagsApplyResponseSchema {
        request: crate::cli::command::agents::tags::apply::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::tags::apply::response_schema::Response>,
    },
    /// `agents tags lookup request_schema` — unary.
    AgentsTagsLookupRequestSchema {
        request: crate::cli::command::agents::tags::lookup::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::tags::lookup::request_schema::Response>,
    },
    /// `agents tags lookup response_schema` — unary.
    AgentsTagsLookupResponseSchema {
        request: crate::cli::command::agents::tags::lookup::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::tags::lookup::response_schema::Response>,
    },
    /// `agents wait` — unary.
    AgentsWait {
        request: crate::cli::command::agents::wait::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::wait::Response>,
    },
    /// `agents wait request_schema` — unary.
    AgentsWaitRequestSchema {
        request: crate::cli::command::agents::wait::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::wait::request_schema::Response>,
    },
    /// `agents wait response_schema` — unary.
    AgentsWaitResponseSchema {
        request: crate::cli::command::agents::wait::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::agents::wait::response_schema::Response>,
    },
    /// `api config address get` — unary.
    ApiConfigAddressGet {
        request: crate::cli::command::api::config::address::get::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::address::get::Response>,
    },
    /// `api config address get request_schema` — unary.
    ApiConfigAddressGetRequestSchema {
        request: crate::cli::command::api::config::address::get::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::address::get::request_schema::Response>,
    },
    /// `api config address get response_schema` — unary.
    ApiConfigAddressGetResponseSchema {
        request: crate::cli::command::api::config::address::get::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::address::get::response_schema::Response>,
    },
    /// `api config address set` — unary.
    ApiConfigAddressSet {
        request: crate::cli::command::api::config::address::set::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::address::set::Response>,
    },
    /// `api config address set request_schema` — unary.
    ApiConfigAddressSetRequestSchema {
        request: crate::cli::command::api::config::address::set::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::address::set::request_schema::Response>,
    },
    /// `api config address set response_schema` — unary.
    ApiConfigAddressSetResponseSchema {
        request: crate::cli::command::api::config::address::set::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::address::set::response_schema::Response>,
    },
    /// `api config backoff_max_elapsed_time_ms get` — unary.
    ApiConfigBackoffMaxElapsedTimeMsGet {
        request: crate::cli::command::api::config::backoff_max_elapsed_time_ms::get::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::backoff_max_elapsed_time_ms::get::Response>,
    },
    /// `api config backoff_max_elapsed_time_ms get request_schema` — unary.
    ApiConfigBackoffMaxElapsedTimeMsGetRequestSchema {
        request: crate::cli::command::api::config::backoff_max_elapsed_time_ms::get::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::backoff_max_elapsed_time_ms::get::request_schema::Response>,
    },
    /// `api config backoff_max_elapsed_time_ms get response_schema` — unary.
    ApiConfigBackoffMaxElapsedTimeMsGetResponseSchema {
        request: crate::cli::command::api::config::backoff_max_elapsed_time_ms::get::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::backoff_max_elapsed_time_ms::get::response_schema::Response>,
    },
    /// `api config backoff_max_elapsed_time_ms set` — unary.
    ApiConfigBackoffMaxElapsedTimeMsSet {
        request: crate::cli::command::api::config::backoff_max_elapsed_time_ms::set::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::backoff_max_elapsed_time_ms::set::Response>,
    },
    /// `api config backoff_max_elapsed_time_ms set request_schema` — unary.
    ApiConfigBackoffMaxElapsedTimeMsSetRequestSchema {
        request: crate::cli::command::api::config::backoff_max_elapsed_time_ms::set::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::backoff_max_elapsed_time_ms::set::request_schema::Response>,
    },
    /// `api config backoff_max_elapsed_time_ms set response_schema` — unary.
    ApiConfigBackoffMaxElapsedTimeMsSetResponseSchema {
        request: crate::cli::command::api::config::backoff_max_elapsed_time_ms::set::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::backoff_max_elapsed_time_ms::set::response_schema::Response>,
    },
    /// `api config commit_author_email get` — unary.
    ApiConfigCommitAuthorEmailGet {
        request: crate::cli::command::api::config::commit_author_email::get::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::commit_author_email::get::Response>,
    },
    /// `api config commit_author_email get request_schema` — unary.
    ApiConfigCommitAuthorEmailGetRequestSchema {
        request: crate::cli::command::api::config::commit_author_email::get::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::commit_author_email::get::request_schema::Response>,
    },
    /// `api config commit_author_email get response_schema` — unary.
    ApiConfigCommitAuthorEmailGetResponseSchema {
        request: crate::cli::command::api::config::commit_author_email::get::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::commit_author_email::get::response_schema::Response>,
    },
    /// `api config commit_author_email set` — unary.
    ApiConfigCommitAuthorEmailSet {
        request: crate::cli::command::api::config::commit_author_email::set::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::commit_author_email::set::Response>,
    },
    /// `api config commit_author_email set request_schema` — unary.
    ApiConfigCommitAuthorEmailSetRequestSchema {
        request: crate::cli::command::api::config::commit_author_email::set::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::commit_author_email::set::request_schema::Response>,
    },
    /// `api config commit_author_email set response_schema` — unary.
    ApiConfigCommitAuthorEmailSetResponseSchema {
        request: crate::cli::command::api::config::commit_author_email::set::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::commit_author_email::set::response_schema::Response>,
    },
    /// `api config commit_author_name get` — unary.
    ApiConfigCommitAuthorNameGet {
        request: crate::cli::command::api::config::commit_author_name::get::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::commit_author_name::get::Response>,
    },
    /// `api config commit_author_name get request_schema` — unary.
    ApiConfigCommitAuthorNameGetRequestSchema {
        request: crate::cli::command::api::config::commit_author_name::get::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::commit_author_name::get::request_schema::Response>,
    },
    /// `api config commit_author_name get response_schema` — unary.
    ApiConfigCommitAuthorNameGetResponseSchema {
        request: crate::cli::command::api::config::commit_author_name::get::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::commit_author_name::get::response_schema::Response>,
    },
    /// `api config commit_author_name set` — unary.
    ApiConfigCommitAuthorNameSet {
        request: crate::cli::command::api::config::commit_author_name::set::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::commit_author_name::set::Response>,
    },
    /// `api config commit_author_name set request_schema` — unary.
    ApiConfigCommitAuthorNameSetRequestSchema {
        request: crate::cli::command::api::config::commit_author_name::set::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::commit_author_name::set::request_schema::Response>,
    },
    /// `api config commit_author_name set response_schema` — unary.
    ApiConfigCommitAuthorNameSetResponseSchema {
        request: crate::cli::command::api::config::commit_author_name::set::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::commit_author_name::set::response_schema::Response>,
    },
    /// `api config get` — unary.
    ApiConfigGet {
        request: crate::cli::command::api::config::get::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::get::Response>,
    },
    /// `api config get request_schema` — unary.
    ApiConfigGetRequestSchema {
        request: crate::cli::command::api::config::get::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::get::request_schema::Response>,
    },
    /// `api config get response_schema` — unary.
    ApiConfigGetResponseSchema {
        request: crate::cli::command::api::config::get::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::get::response_schema::Response>,
    },
    /// `api config github_authorization get` — unary.
    ApiConfigGithubAuthorizationGet {
        request: crate::cli::command::api::config::github_authorization::get::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::github_authorization::get::Response>,
    },
    /// `api config github_authorization get request_schema` — unary.
    ApiConfigGithubAuthorizationGetRequestSchema {
        request: crate::cli::command::api::config::github_authorization::get::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::github_authorization::get::request_schema::Response>,
    },
    /// `api config github_authorization get response_schema` — unary.
    ApiConfigGithubAuthorizationGetResponseSchema {
        request: crate::cli::command::api::config::github_authorization::get::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::github_authorization::get::response_schema::Response>,
    },
    /// `api config github_authorization set` — unary.
    ApiConfigGithubAuthorizationSet {
        request: crate::cli::command::api::config::github_authorization::set::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::github_authorization::set::Response>,
    },
    /// `api config github_authorization set request_schema` — unary.
    ApiConfigGithubAuthorizationSetRequestSchema {
        request: crate::cli::command::api::config::github_authorization::set::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::github_authorization::set::request_schema::Response>,
    },
    /// `api config github_authorization set response_schema` — unary.
    ApiConfigGithubAuthorizationSetResponseSchema {
        request: crate::cli::command::api::config::github_authorization::set::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::github_authorization::set::response_schema::Response>,
    },
    /// `api config http_referer get` — unary.
    ApiConfigHttpRefererGet {
        request: crate::cli::command::api::config::http_referer::get::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::http_referer::get::Response>,
    },
    /// `api config http_referer get request_schema` — unary.
    ApiConfigHttpRefererGetRequestSchema {
        request: crate::cli::command::api::config::http_referer::get::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::http_referer::get::request_schema::Response>,
    },
    /// `api config http_referer get response_schema` — unary.
    ApiConfigHttpRefererGetResponseSchema {
        request: crate::cli::command::api::config::http_referer::get::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::http_referer::get::response_schema::Response>,
    },
    /// `api config http_referer set` — unary.
    ApiConfigHttpRefererSet {
        request: crate::cli::command::api::config::http_referer::set::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::http_referer::set::Response>,
    },
    /// `api config http_referer set request_schema` — unary.
    ApiConfigHttpRefererSetRequestSchema {
        request: crate::cli::command::api::config::http_referer::set::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::http_referer::set::request_schema::Response>,
    },
    /// `api config http_referer set response_schema` — unary.
    ApiConfigHttpRefererSetResponseSchema {
        request: crate::cli::command::api::config::http_referer::set::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::http_referer::set::response_schema::Response>,
    },
    /// `api config mcp_authorization add` — unary.
    ApiConfigMcpAuthorizationAdd {
        request: crate::cli::command::api::config::mcp_authorization::add::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::mcp_authorization::add::Response>,
    },
    /// `api config mcp_authorization add request_schema` — unary.
    ApiConfigMcpAuthorizationAddRequestSchema {
        request: crate::cli::command::api::config::mcp_authorization::add::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::mcp_authorization::add::request_schema::Response>,
    },
    /// `api config mcp_authorization add response_schema` — unary.
    ApiConfigMcpAuthorizationAddResponseSchema {
        request: crate::cli::command::api::config::mcp_authorization::add::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::mcp_authorization::add::response_schema::Response>,
    },
    /// `api config mcp_authorization del` — unary.
    ApiConfigMcpAuthorizationDel {
        request: crate::cli::command::api::config::mcp_authorization::del::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::mcp_authorization::del::Response>,
    },
    /// `api config mcp_authorization del request_schema` — unary.
    ApiConfigMcpAuthorizationDelRequestSchema {
        request: crate::cli::command::api::config::mcp_authorization::del::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::mcp_authorization::del::request_schema::Response>,
    },
    /// `api config mcp_authorization del response_schema` — unary.
    ApiConfigMcpAuthorizationDelResponseSchema {
        request: crate::cli::command::api::config::mcp_authorization::del::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::mcp_authorization::del::response_schema::Response>,
    },
    /// `api config mcp_authorization get` — unary.
    ApiConfigMcpAuthorizationGet {
        request: crate::cli::command::api::config::mcp_authorization::get::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::mcp_authorization::get::Response>,
    },
    /// `api config mcp_authorization get request_schema` — unary.
    ApiConfigMcpAuthorizationGetRequestSchema {
        request: crate::cli::command::api::config::mcp_authorization::get::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::mcp_authorization::get::request_schema::Response>,
    },
    /// `api config mcp_authorization get response_schema` — unary.
    ApiConfigMcpAuthorizationGetResponseSchema {
        request: crate::cli::command::api::config::mcp_authorization::get::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::mcp_authorization::get::response_schema::Response>,
    },
    /// `api config mcp_timeout_ms get` — unary.
    ApiConfigMcpTimeoutMsGet {
        request: crate::cli::command::api::config::mcp_timeout_ms::get::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::mcp_timeout_ms::get::Response>,
    },
    /// `api config mcp_timeout_ms get request_schema` — unary.
    ApiConfigMcpTimeoutMsGetRequestSchema {
        request: crate::cli::command::api::config::mcp_timeout_ms::get::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::mcp_timeout_ms::get::request_schema::Response>,
    },
    /// `api config mcp_timeout_ms get response_schema` — unary.
    ApiConfigMcpTimeoutMsGetResponseSchema {
        request: crate::cli::command::api::config::mcp_timeout_ms::get::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::mcp_timeout_ms::get::response_schema::Response>,
    },
    /// `api config mcp_timeout_ms set` — unary.
    ApiConfigMcpTimeoutMsSet {
        request: crate::cli::command::api::config::mcp_timeout_ms::set::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::mcp_timeout_ms::set::Response>,
    },
    /// `api config mcp_timeout_ms set request_schema` — unary.
    ApiConfigMcpTimeoutMsSetRequestSchema {
        request: crate::cli::command::api::config::mcp_timeout_ms::set::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::mcp_timeout_ms::set::request_schema::Response>,
    },
    /// `api config mcp_timeout_ms set response_schema` — unary.
    ApiConfigMcpTimeoutMsSetResponseSchema {
        request: crate::cli::command::api::config::mcp_timeout_ms::set::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::mcp_timeout_ms::set::response_schema::Response>,
    },
    /// `api config objectiveai_authorization get` — unary.
    ApiConfigObjectiveaiAuthorizationGet {
        request: crate::cli::command::api::config::objectiveai_authorization::get::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::objectiveai_authorization::get::Response>,
    },
    /// `api config objectiveai_authorization get request_schema` — unary.
    ApiConfigObjectiveaiAuthorizationGetRequestSchema {
        request: crate::cli::command::api::config::objectiveai_authorization::get::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::objectiveai_authorization::get::request_schema::Response>,
    },
    /// `api config objectiveai_authorization get response_schema` — unary.
    ApiConfigObjectiveaiAuthorizationGetResponseSchema {
        request: crate::cli::command::api::config::objectiveai_authorization::get::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::objectiveai_authorization::get::response_schema::Response>,
    },
    /// `api config objectiveai_authorization set` — unary.
    ApiConfigObjectiveaiAuthorizationSet {
        request: crate::cli::command::api::config::objectiveai_authorization::set::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::objectiveai_authorization::set::Response>,
    },
    /// `api config objectiveai_authorization set request_schema` — unary.
    ApiConfigObjectiveaiAuthorizationSetRequestSchema {
        request: crate::cli::command::api::config::objectiveai_authorization::set::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::objectiveai_authorization::set::request_schema::Response>,
    },
    /// `api config objectiveai_authorization set response_schema` — unary.
    ApiConfigObjectiveaiAuthorizationSetResponseSchema {
        request: crate::cli::command::api::config::objectiveai_authorization::set::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::objectiveai_authorization::set::response_schema::Response>,
    },
    /// `api config openrouter_authorization get` — unary.
    ApiConfigOpenrouterAuthorizationGet {
        request: crate::cli::command::api::config::openrouter_authorization::get::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::openrouter_authorization::get::Response>,
    },
    /// `api config openrouter_authorization get request_schema` — unary.
    ApiConfigOpenrouterAuthorizationGetRequestSchema {
        request: crate::cli::command::api::config::openrouter_authorization::get::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::openrouter_authorization::get::request_schema::Response>,
    },
    /// `api config openrouter_authorization get response_schema` — unary.
    ApiConfigOpenrouterAuthorizationGetResponseSchema {
        request: crate::cli::command::api::config::openrouter_authorization::get::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::openrouter_authorization::get::response_schema::Response>,
    },
    /// `api config openrouter_authorization set` — unary.
    ApiConfigOpenrouterAuthorizationSet {
        request: crate::cli::command::api::config::openrouter_authorization::set::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::openrouter_authorization::set::Response>,
    },
    /// `api config openrouter_authorization set request_schema` — unary.
    ApiConfigOpenrouterAuthorizationSetRequestSchema {
        request: crate::cli::command::api::config::openrouter_authorization::set::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::openrouter_authorization::set::request_schema::Response>,
    },
    /// `api config openrouter_authorization set response_schema` — unary.
    ApiConfigOpenrouterAuthorizationSetResponseSchema {
        request: crate::cli::command::api::config::openrouter_authorization::set::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::openrouter_authorization::set::response_schema::Response>,
    },
    /// `api config user_agent get` — unary.
    ApiConfigUserAgentGet {
        request: crate::cli::command::api::config::user_agent::get::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::user_agent::get::Response>,
    },
    /// `api config user_agent get request_schema` — unary.
    ApiConfigUserAgentGetRequestSchema {
        request: crate::cli::command::api::config::user_agent::get::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::user_agent::get::request_schema::Response>,
    },
    /// `api config user_agent get response_schema` — unary.
    ApiConfigUserAgentGetResponseSchema {
        request: crate::cli::command::api::config::user_agent::get::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::user_agent::get::response_schema::Response>,
    },
    /// `api config user_agent set` — unary.
    ApiConfigUserAgentSet {
        request: crate::cli::command::api::config::user_agent::set::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::user_agent::set::Response>,
    },
    /// `api config user_agent set request_schema` — unary.
    ApiConfigUserAgentSetRequestSchema {
        request: crate::cli::command::api::config::user_agent::set::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::user_agent::set::request_schema::Response>,
    },
    /// `api config user_agent set response_schema` — unary.
    ApiConfigUserAgentSetResponseSchema {
        request: crate::cli::command::api::config::user_agent::set::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::user_agent::set::response_schema::Response>,
    },
    /// `api config x_title get` — unary.
    ApiConfigXTitleGet {
        request: crate::cli::command::api::config::x_title::get::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::x_title::get::Response>,
    },
    /// `api config x_title get request_schema` — unary.
    ApiConfigXTitleGetRequestSchema {
        request: crate::cli::command::api::config::x_title::get::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::x_title::get::request_schema::Response>,
    },
    /// `api config x_title get response_schema` — unary.
    ApiConfigXTitleGetResponseSchema {
        request: crate::cli::command::api::config::x_title::get::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::x_title::get::response_schema::Response>,
    },
    /// `api config x_title set` — unary.
    ApiConfigXTitleSet {
        request: crate::cli::command::api::config::x_title::set::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::x_title::set::Response>,
    },
    /// `api config x_title set request_schema` — unary.
    ApiConfigXTitleSetRequestSchema {
        request: crate::cli::command::api::config::x_title::set::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::x_title::set::request_schema::Response>,
    },
    /// `api config x_title set response_schema` — unary.
    ApiConfigXTitleSetResponseSchema {
        request: crate::cli::command::api::config::x_title::set::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::config::x_title::set::response_schema::Response>,
    },
    /// `api kill` — unary.
    ApiKill {
        request: crate::cli::command::api::kill::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::kill::Response>,
    },
    /// `api kill request_schema` — unary.
    ApiKillRequestSchema {
        request: crate::cli::command::api::kill::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::kill::request_schema::Response>,
    },
    /// `api kill response_schema` — unary.
    ApiKillResponseSchema {
        request: crate::cli::command::api::kill::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::kill::response_schema::Response>,
    },
    /// `api spawn` — unary.
    ApiSpawn {
        request: crate::cli::command::api::spawn::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::spawn::Response>,
    },
    /// `api spawn request_schema` — unary.
    ApiSpawnRequestSchema {
        request: crate::cli::command::api::spawn::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::spawn::request_schema::Response>,
    },
    /// `api spawn response_schema` — unary.
    ApiSpawnResponseSchema {
        request: crate::cli::command::api::spawn::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::api::spawn::response_schema::Response>,
    },
    /// `daemon kill` — unary.
    DaemonKill {
        request: crate::cli::command::daemon::kill::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::daemon::kill::Response>,
    },
    /// `daemon kill request_schema` — unary.
    DaemonKillRequestSchema {
        request: crate::cli::command::daemon::kill::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::daemon::kill::request_schema::Response>,
    },
    /// `daemon kill response_schema` — unary.
    DaemonKillResponseSchema {
        request: crate::cli::command::daemon::kill::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::daemon::kill::response_schema::Response>,
    },
    /// `daemon spawn` — streaming.
    DaemonSpawn {
        request: crate::cli::command::daemon::spawn::Request,
        agent_arguments: AgentArguments,
        response: ResponseItemStream<crate::cli::command::daemon::spawn::ResponseItem>,
    },
    /// `daemon spawn request_schema` — unary.
    DaemonSpawnRequestSchema {
        request: crate::cli::command::daemon::spawn::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::daemon::spawn::request_schema::Response>,
    },
    /// `daemon spawn response_schema` — unary.
    DaemonSpawnResponseSchema {
        request: crate::cli::command::daemon::spawn::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::daemon::spawn::response_schema::Response>,
    },
    /// `db config address get` — unary.
    DbConfigAddressGet {
        request: crate::cli::command::db::config::address::get::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::db::config::address::get::Response>,
    },
    /// `db config address get request_schema` — unary.
    DbConfigAddressGetRequestSchema {
        request: crate::cli::command::db::config::address::get::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::db::config::address::get::request_schema::Response>,
    },
    /// `db config address get response_schema` — unary.
    DbConfigAddressGetResponseSchema {
        request: crate::cli::command::db::config::address::get::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::db::config::address::get::response_schema::Response>,
    },
    /// `db config address set` — unary.
    DbConfigAddressSet {
        request: crate::cli::command::db::config::address::set::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::db::config::address::set::Response>,
    },
    /// `db config address set request_schema` — unary.
    DbConfigAddressSetRequestSchema {
        request: crate::cli::command::db::config::address::set::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::db::config::address::set::request_schema::Response>,
    },
    /// `db config address set response_schema` — unary.
    DbConfigAddressSetResponseSchema {
        request: crate::cli::command::db::config::address::set::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::db::config::address::set::response_schema::Response>,
    },
    /// `db config database get` — unary.
    DbConfigDatabaseGet {
        request: crate::cli::command::db::config::database::get::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::db::config::database::get::Response>,
    },
    /// `db config database get request_schema` — unary.
    DbConfigDatabaseGetRequestSchema {
        request: crate::cli::command::db::config::database::get::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::db::config::database::get::request_schema::Response>,
    },
    /// `db config database get response_schema` — unary.
    DbConfigDatabaseGetResponseSchema {
        request: crate::cli::command::db::config::database::get::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::db::config::database::get::response_schema::Response>,
    },
    /// `db config database set` — unary.
    DbConfigDatabaseSet {
        request: crate::cli::command::db::config::database::set::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::db::config::database::set::Response>,
    },
    /// `db config database set request_schema` — unary.
    DbConfigDatabaseSetRequestSchema {
        request: crate::cli::command::db::config::database::set::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::db::config::database::set::request_schema::Response>,
    },
    /// `db config database set response_schema` — unary.
    DbConfigDatabaseSetResponseSchema {
        request: crate::cli::command::db::config::database::set::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::db::config::database::set::response_schema::Response>,
    },
    /// `db config get` — unary.
    DbConfigGet {
        request: crate::cli::command::db::config::get::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::db::config::get::Response>,
    },
    /// `db config get request_schema` — unary.
    DbConfigGetRequestSchema {
        request: crate::cli::command::db::config::get::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::db::config::get::request_schema::Response>,
    },
    /// `db config get response_schema` — unary.
    DbConfigGetResponseSchema {
        request: crate::cli::command::db::config::get::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::db::config::get::response_schema::Response>,
    },
    /// `db config password get` — unary.
    DbConfigPasswordGet {
        request: crate::cli::command::db::config::password::get::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::db::config::password::get::Response>,
    },
    /// `db config password get request_schema` — unary.
    DbConfigPasswordGetRequestSchema {
        request: crate::cli::command::db::config::password::get::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::db::config::password::get::request_schema::Response>,
    },
    /// `db config password get response_schema` — unary.
    DbConfigPasswordGetResponseSchema {
        request: crate::cli::command::db::config::password::get::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::db::config::password::get::response_schema::Response>,
    },
    /// `db config password set` — unary.
    DbConfigPasswordSet {
        request: crate::cli::command::db::config::password::set::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::db::config::password::set::Response>,
    },
    /// `db config password set request_schema` — unary.
    DbConfigPasswordSetRequestSchema {
        request: crate::cli::command::db::config::password::set::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::db::config::password::set::request_schema::Response>,
    },
    /// `db config password set response_schema` — unary.
    DbConfigPasswordSetResponseSchema {
        request: crate::cli::command::db::config::password::set::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::db::config::password::set::response_schema::Response>,
    },
    /// `db config user get` — unary.
    DbConfigUserGet {
        request: crate::cli::command::db::config::user::get::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::db::config::user::get::Response>,
    },
    /// `db config user get request_schema` — unary.
    DbConfigUserGetRequestSchema {
        request: crate::cli::command::db::config::user::get::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::db::config::user::get::request_schema::Response>,
    },
    /// `db config user get response_schema` — unary.
    DbConfigUserGetResponseSchema {
        request: crate::cli::command::db::config::user::get::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::db::config::user::get::response_schema::Response>,
    },
    /// `db config user set` — unary.
    DbConfigUserSet {
        request: crate::cli::command::db::config::user::set::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::db::config::user::set::Response>,
    },
    /// `db config user set request_schema` — unary.
    DbConfigUserSetRequestSchema {
        request: crate::cli::command::db::config::user::set::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::db::config::user::set::request_schema::Response>,
    },
    /// `db config user set response_schema` — unary.
    DbConfigUserSetResponseSchema {
        request: crate::cli::command::db::config::user::set::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::db::config::user::set::response_schema::Response>,
    },
    /// `db kill` — unary.
    DbKill {
        request: crate::cli::command::db::kill::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::db::kill::Response>,
    },
    /// `db kill request_schema` — unary.
    DbKillRequestSchema {
        request: crate::cli::command::db::kill::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::db::kill::request_schema::Response>,
    },
    /// `db kill response_schema` — unary.
    DbKillResponseSchema {
        request: crate::cli::command::db::kill::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::db::kill::response_schema::Response>,
    },
    /// `db query` — unary.
    DbQuery {
        request: crate::cli::command::db::query::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::db::query::Response>,
    },
    /// `db query request_schema` — unary.
    DbQueryRequestSchema {
        request: crate::cli::command::db::query::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::db::query::request_schema::Response>,
    },
    /// `db query response_schema` — unary.
    DbQueryResponseSchema {
        request: crate::cli::command::db::query::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::db::query::response_schema::Response>,
    },
    /// `db spawn` — unary.
    DbSpawn {
        request: crate::cli::command::db::spawn::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::db::spawn::Response>,
    },
    /// `db spawn request_schema` — unary.
    DbSpawnRequestSchema {
        request: crate::cli::command::db::spawn::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::db::spawn::request_schema::Response>,
    },
    /// `db spawn response_schema` — unary.
    DbSpawnResponseSchema {
        request: crate::cli::command::db::spawn::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::db::spawn::response_schema::Response>,
    },
    /// `functions execute standard` — unary form.
    FunctionsExecuteStandard {
        request: crate::cli::command::functions::execute::standard::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::functions::execute::standard::Response>,
    },
    /// `functions execute standard` — streaming form (`dangerous_advanced.stream: true`).
    FunctionsExecuteStandardStreaming {
        request: crate::cli::command::functions::execute::standard::Request,
        agent_arguments: AgentArguments,
        response: ResponseItemStream<crate::cli::command::functions::execute::standard::ResponseItem>,
    },
    /// `functions execute standard request_schema` — unary.
    FunctionsExecuteStandardRequestSchema {
        request: crate::cli::command::functions::execute::standard::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::functions::execute::standard::request_schema::Response>,
    },
    /// `functions execute standard response_schema` — unary.
    FunctionsExecuteStandardResponseSchema {
        request: crate::cli::command::functions::execute::standard::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::functions::execute::standard::response_schema::Response>,
    },
    /// `functions execute swiss_system` — unary form.
    FunctionsExecuteSwissSystem {
        request: crate::cli::command::functions::execute::swiss_system::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::functions::execute::swiss_system::Response>,
    },
    /// `functions execute swiss_system` — streaming form (`dangerous_advanced.stream: true`).
    FunctionsExecuteSwissSystemStreaming {
        request: crate::cli::command::functions::execute::swiss_system::Request,
        agent_arguments: AgentArguments,
        response: ResponseItemStream<crate::cli::command::functions::execute::swiss_system::ResponseItem>,
    },
    /// `functions execute swiss_system request_schema` — unary.
    FunctionsExecuteSwissSystemRequestSchema {
        request: crate::cli::command::functions::execute::swiss_system::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::functions::execute::swiss_system::request_schema::Response>,
    },
    /// `functions execute swiss_system response_schema` — unary.
    FunctionsExecuteSwissSystemResponseSchema {
        request: crate::cli::command::functions::execute::swiss_system::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::functions::execute::swiss_system::response_schema::Response>,
    },
    /// `functions get` — unary.
    FunctionsGet {
        request: crate::cli::command::functions::get::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::functions::get::Response>,
    },
    /// `functions get request_schema` — unary.
    FunctionsGetRequestSchema {
        request: crate::cli::command::functions::get::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::functions::get::request_schema::Response>,
    },
    /// `functions get response_schema` — unary.
    FunctionsGetResponseSchema {
        request: crate::cli::command::functions::get::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::functions::get::response_schema::Response>,
    },
    /// `functions list` — streaming.
    FunctionsList {
        request: crate::cli::command::functions::list::Request,
        agent_arguments: AgentArguments,
        response: ResponseItemStream<crate::cli::command::functions::list::ResponseItem>,
    },
    /// `functions list request_schema` — unary.
    FunctionsListRequestSchema {
        request: crate::cli::command::functions::list::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::functions::list::request_schema::Response>,
    },
    /// `functions list response_schema` — unary.
    FunctionsListResponseSchema {
        request: crate::cli::command::functions::list::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::functions::list::response_schema::Response>,
    },
    /// `functions profiles get` — unary.
    FunctionsProfilesGet {
        request: crate::cli::command::functions::profiles::get::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::functions::profiles::get::Response>,
    },
    /// `functions profiles get request_schema` — unary.
    FunctionsProfilesGetRequestSchema {
        request: crate::cli::command::functions::profiles::get::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::functions::profiles::get::request_schema::Response>,
    },
    /// `functions profiles get response_schema` — unary.
    FunctionsProfilesGetResponseSchema {
        request: crate::cli::command::functions::profiles::get::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::functions::profiles::get::response_schema::Response>,
    },
    /// `functions profiles list` — streaming.
    FunctionsProfilesList {
        request: crate::cli::command::functions::profiles::list::Request,
        agent_arguments: AgentArguments,
        response: ResponseItemStream<crate::cli::command::functions::profiles::list::ResponseItem>,
    },
    /// `functions profiles list request_schema` — unary.
    FunctionsProfilesListRequestSchema {
        request: crate::cli::command::functions::profiles::list::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::functions::profiles::list::request_schema::Response>,
    },
    /// `functions profiles list response_schema` — unary.
    FunctionsProfilesListResponseSchema {
        request: crate::cli::command::functions::profiles::list::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::functions::profiles::list::response_schema::Response>,
    },
    /// `functions profiles publish` — unary.
    FunctionsProfilesPublish {
        request: crate::cli::command::functions::profiles::publish::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::functions::profiles::publish::Response>,
    },
    /// `functions profiles publish request_schema` — unary.
    FunctionsProfilesPublishRequestSchema {
        request: crate::cli::command::functions::profiles::publish::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::functions::profiles::publish::request_schema::Response>,
    },
    /// `functions profiles publish response_schema` — unary.
    FunctionsProfilesPublishResponseSchema {
        request: crate::cli::command::functions::profiles::publish::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::functions::profiles::publish::response_schema::Response>,
    },
    /// `functions publish` — unary.
    FunctionsPublish {
        request: crate::cli::command::functions::publish::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::functions::publish::Response>,
    },
    /// `functions publish request_schema` — unary.
    FunctionsPublishRequestSchema {
        request: crate::cli::command::functions::publish::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::functions::publish::request_schema::Response>,
    },
    /// `functions publish response_schema` — unary.
    FunctionsPublishResponseSchema {
        request: crate::cli::command::functions::publish::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::functions::publish::response_schema::Response>,
    },
    /// `kill_all` — unary.
    KillAll {
        request: crate::cli::command::kill_all::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::kill_all::Response>,
    },
    /// `kill_all request_schema` — unary.
    KillAllRequestSchema {
        request: crate::cli::command::kill_all::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::kill_all::request_schema::Response>,
    },
    /// `kill_all response_schema` — unary.
    KillAllResponseSchema {
        request: crate::cli::command::kill_all::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::kill_all::response_schema::Response>,
    },
    /// `laboratories create` — unary.
    LaboratoriesCreate {
        request: crate::cli::command::laboratories::create::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::laboratories::create::Response>,
    },
    /// `laboratories create request_schema` — unary.
    LaboratoriesCreateRequestSchema {
        request: crate::cli::command::laboratories::create::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::laboratories::create::request_schema::Response>,
    },
    /// `laboratories create response_schema` — unary.
    LaboratoriesCreateResponseSchema {
        request: crate::cli::command::laboratories::create::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::laboratories::create::response_schema::Response>,
    },
    /// `laboratories list` — streaming.
    LaboratoriesList {
        request: crate::cli::command::laboratories::list::Request,
        agent_arguments: AgentArguments,
        response: ResponseItemStream<crate::cli::command::laboratories::list::ResponseItem>,
    },
    /// `laboratories list request_schema` — unary.
    LaboratoriesListRequestSchema {
        request: crate::cli::command::laboratories::list::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::laboratories::list::request_schema::Response>,
    },
    /// `laboratories list response_schema` — unary.
    LaboratoriesListResponseSchema {
        request: crate::cli::command::laboratories::list::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::laboratories::list::response_schema::Response>,
    },
    /// `mcp config address get` — unary.
    McpConfigAddressGet {
        request: crate::cli::command::mcp::config::address::get::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::mcp::config::address::get::Response>,
    },
    /// `mcp config address get request_schema` — unary.
    McpConfigAddressGetRequestSchema {
        request: crate::cli::command::mcp::config::address::get::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::mcp::config::address::get::request_schema::Response>,
    },
    /// `mcp config address get response_schema` — unary.
    McpConfigAddressGetResponseSchema {
        request: crate::cli::command::mcp::config::address::get::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::mcp::config::address::get::response_schema::Response>,
    },
    /// `mcp config address set` — unary.
    McpConfigAddressSet {
        request: crate::cli::command::mcp::config::address::set::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::mcp::config::address::set::Response>,
    },
    /// `mcp config address set request_schema` — unary.
    McpConfigAddressSetRequestSchema {
        request: crate::cli::command::mcp::config::address::set::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::mcp::config::address::set::request_schema::Response>,
    },
    /// `mcp config address set response_schema` — unary.
    McpConfigAddressSetResponseSchema {
        request: crate::cli::command::mcp::config::address::set::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::mcp::config::address::set::response_schema::Response>,
    },
    /// `mcp config get` — unary.
    McpConfigGet {
        request: crate::cli::command::mcp::config::get::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::mcp::config::get::Response>,
    },
    /// `mcp config get request_schema` — unary.
    McpConfigGetRequestSchema {
        request: crate::cli::command::mcp::config::get::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::mcp::config::get::request_schema::Response>,
    },
    /// `mcp config get response_schema` — unary.
    McpConfigGetResponseSchema {
        request: crate::cli::command::mcp::config::get::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::mcp::config::get::response_schema::Response>,
    },
    /// `mcp config port get` — unary.
    McpConfigPortGet {
        request: crate::cli::command::mcp::config::port::get::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::mcp::config::port::get::Response>,
    },
    /// `mcp config port get request_schema` — unary.
    McpConfigPortGetRequestSchema {
        request: crate::cli::command::mcp::config::port::get::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::mcp::config::port::get::request_schema::Response>,
    },
    /// `mcp config port get response_schema` — unary.
    McpConfigPortGetResponseSchema {
        request: crate::cli::command::mcp::config::port::get::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::mcp::config::port::get::response_schema::Response>,
    },
    /// `mcp config port set` — unary.
    McpConfigPortSet {
        request: crate::cli::command::mcp::config::port::set::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::mcp::config::port::set::Response>,
    },
    /// `mcp config port set request_schema` — unary.
    McpConfigPortSetRequestSchema {
        request: crate::cli::command::mcp::config::port::set::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::mcp::config::port::set::request_schema::Response>,
    },
    /// `mcp config port set response_schema` — unary.
    McpConfigPortSetResponseSchema {
        request: crate::cli::command::mcp::config::port::set::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::mcp::config::port::set::response_schema::Response>,
    },
    /// `mcp kill` — unary.
    McpKill {
        request: crate::cli::command::mcp::kill::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::mcp::kill::Response>,
    },
    /// `mcp kill request_schema` — unary.
    McpKillRequestSchema {
        request: crate::cli::command::mcp::kill::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::mcp::kill::request_schema::Response>,
    },
    /// `mcp kill response_schema` — unary.
    McpKillResponseSchema {
        request: crate::cli::command::mcp::kill::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::mcp::kill::response_schema::Response>,
    },
    /// `mcp spawn` — unary.
    McpSpawn {
        request: crate::cli::command::mcp::spawn::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::mcp::spawn::Response>,
    },
    /// `mcp spawn request_schema` — unary.
    McpSpawnRequestSchema {
        request: crate::cli::command::mcp::spawn::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::mcp::spawn::request_schema::Response>,
    },
    /// `mcp spawn response_schema` — unary.
    McpSpawnResponseSchema {
        request: crate::cli::command::mcp::spawn::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::mcp::spawn::response_schema::Response>,
    },
    /// `plugins get` — unary.
    PluginsGet {
        request: crate::cli::command::plugins::get::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::plugins::get::Response>,
    },
    /// `plugins get request_schema` — unary.
    PluginsGetRequestSchema {
        request: crate::cli::command::plugins::get::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::plugins::get::request_schema::Response>,
    },
    /// `plugins get response_schema` — unary.
    PluginsGetResponseSchema {
        request: crate::cli::command::plugins::get::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::plugins::get::response_schema::Response>,
    },
    /// `plugins install filesystem` — unary.
    PluginsInstallFilesystem {
        request: crate::cli::command::plugins::install::filesystem::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::plugins::install::filesystem::Response>,
    },
    /// `plugins install filesystem request_schema` — unary.
    PluginsInstallFilesystemRequestSchema {
        request: crate::cli::command::plugins::install::filesystem::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::plugins::install::filesystem::request_schema::Response>,
    },
    /// `plugins install filesystem response_schema` — unary.
    PluginsInstallFilesystemResponseSchema {
        request: crate::cli::command::plugins::install::filesystem::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::plugins::install::filesystem::response_schema::Response>,
    },
    /// `plugins install github` — unary.
    PluginsInstallGithub {
        request: crate::cli::command::plugins::install::github::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::plugins::install::github::Response>,
    },
    /// `plugins install github request_schema` — unary.
    PluginsInstallGithubRequestSchema {
        request: crate::cli::command::plugins::install::github::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::plugins::install::github::request_schema::Response>,
    },
    /// `plugins install github response_schema` — unary.
    PluginsInstallGithubResponseSchema {
        request: crate::cli::command::plugins::install::github::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::plugins::install::github::response_schema::Response>,
    },
    /// `plugins list` — streaming.
    PluginsList {
        request: crate::cli::command::plugins::list::Request,
        agent_arguments: AgentArguments,
        response: ResponseItemStream<crate::cli::command::plugins::list::ResponseItem>,
    },
    /// `plugins list request_schema` — unary.
    PluginsListRequestSchema {
        request: crate::cli::command::plugins::list::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::plugins::list::request_schema::Response>,
    },
    /// `plugins list response_schema` — unary.
    PluginsListResponseSchema {
        request: crate::cli::command::plugins::list::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::plugins::list::response_schema::Response>,
    },
    /// `plugins logs list` — streaming.
    PluginsLogsList {
        request: crate::cli::command::plugins::logs::list::Request,
        agent_arguments: AgentArguments,
        response: ResponseItemStream<crate::cli::command::plugins::logs::list::ResponseItem>,
    },
    /// `plugins logs list request_schema` — unary.
    PluginsLogsListRequestSchema {
        request: crate::cli::command::plugins::logs::list::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::plugins::logs::list::request_schema::Response>,
    },
    /// `plugins logs list response_schema` — unary.
    PluginsLogsListResponseSchema {
        request: crate::cli::command::plugins::logs::list::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::plugins::logs::list::response_schema::Response>,
    },
    /// `plugins run` — streaming.
    PluginsRun {
        request: crate::cli::command::plugins::run::Request,
        agent_arguments: AgentArguments,
        response: ResponseItemStream<crate::cli::command::plugins::run::ResponseItem>,
    },
    /// `plugins run request_schema` — unary.
    PluginsRunRequestSchema {
        request: crate::cli::command::plugins::run::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::plugins::run::request_schema::Response>,
    },
    /// `plugins run response_schema` — unary.
    PluginsRunResponseSchema {
        request: crate::cli::command::plugins::run::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::plugins::run::response_schema::Response>,
    },
    /// `python` — unary.
    Python {
        request: crate::cli::command::python::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::python::Response>,
    },
    /// `python request_schema` — unary.
    PythonRequestSchema {
        request: crate::cli::command::python::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::python::request_schema::Response>,
    },
    /// `python response_schema` — unary.
    PythonResponseSchema {
        request: crate::cli::command::python::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::python::response_schema::Response>,
    },
    /// `swarms get` — unary.
    SwarmsGet {
        request: crate::cli::command::swarms::get::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::swarms::get::Response>,
    },
    /// `swarms get request_schema` — unary.
    SwarmsGetRequestSchema {
        request: crate::cli::command::swarms::get::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::swarms::get::request_schema::Response>,
    },
    /// `swarms get response_schema` — unary.
    SwarmsGetResponseSchema {
        request: crate::cli::command::swarms::get::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::swarms::get::response_schema::Response>,
    },
    /// `swarms list` — streaming.
    SwarmsList {
        request: crate::cli::command::swarms::list::Request,
        agent_arguments: AgentArguments,
        response: ResponseItemStream<crate::cli::command::swarms::list::ResponseItem>,
    },
    /// `swarms list request_schema` — unary.
    SwarmsListRequestSchema {
        request: crate::cli::command::swarms::list::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::swarms::list::request_schema::Response>,
    },
    /// `swarms list response_schema` — unary.
    SwarmsListResponseSchema {
        request: crate::cli::command::swarms::list::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::swarms::list::response_schema::Response>,
    },
    /// `swarms publish` — unary.
    SwarmsPublish {
        request: crate::cli::command::swarms::publish::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::swarms::publish::Response>,
    },
    /// `swarms publish request_schema` — unary.
    SwarmsPublishRequestSchema {
        request: crate::cli::command::swarms::publish::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::swarms::publish::request_schema::Response>,
    },
    /// `swarms publish response_schema` — unary.
    SwarmsPublishResponseSchema {
        request: crate::cli::command::swarms::publish::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::swarms::publish::response_schema::Response>,
    },
    /// `tools get` — unary.
    ToolsGet {
        request: crate::cli::command::tools::get::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::tools::get::Response>,
    },
    /// `tools get request_schema` — unary.
    ToolsGetRequestSchema {
        request: crate::cli::command::tools::get::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::tools::get::request_schema::Response>,
    },
    /// `tools get response_schema` — unary.
    ToolsGetResponseSchema {
        request: crate::cli::command::tools::get::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::tools::get::response_schema::Response>,
    },
    /// `tools install filesystem` — unary.
    ToolsInstallFilesystem {
        request: crate::cli::command::tools::install::filesystem::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::tools::install::filesystem::Response>,
    },
    /// `tools install filesystem request_schema` — unary.
    ToolsInstallFilesystemRequestSchema {
        request: crate::cli::command::tools::install::filesystem::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::tools::install::filesystem::request_schema::Response>,
    },
    /// `tools install filesystem response_schema` — unary.
    ToolsInstallFilesystemResponseSchema {
        request: crate::cli::command::tools::install::filesystem::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::tools::install::filesystem::response_schema::Response>,
    },
    /// `tools install github` — unary.
    ToolsInstallGithub {
        request: crate::cli::command::tools::install::github::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::tools::install::github::Response>,
    },
    /// `tools install github request_schema` — unary.
    ToolsInstallGithubRequestSchema {
        request: crate::cli::command::tools::install::github::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::tools::install::github::request_schema::Response>,
    },
    /// `tools install github response_schema` — unary.
    ToolsInstallGithubResponseSchema {
        request: crate::cli::command::tools::install::github::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::tools::install::github::response_schema::Response>,
    },
    /// `tools list` — streaming.
    ToolsList {
        request: crate::cli::command::tools::list::Request,
        agent_arguments: AgentArguments,
        response: ResponseItemStream<crate::cli::command::tools::list::ResponseItem>,
    },
    /// `tools list request_schema` — unary.
    ToolsListRequestSchema {
        request: crate::cli::command::tools::list::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::tools::list::request_schema::Response>,
    },
    /// `tools list response_schema` — unary.
    ToolsListResponseSchema {
        request: crate::cli::command::tools::list::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::tools::list::response_schema::Response>,
    },
    /// `tools run` — streaming.
    ToolsRun {
        request: crate::cli::command::tools::run::Request,
        agent_arguments: AgentArguments,
        response: ResponseItemStream<crate::cli::command::tools::run::ResponseItem>,
    },
    /// `tools run request_schema` — unary.
    ToolsRunRequestSchema {
        request: crate::cli::command::tools::run::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::tools::run::request_schema::Response>,
    },
    /// `tools run response_schema` — unary.
    ToolsRunResponseSchema {
        request: crate::cli::command::tools::run::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::tools::run::response_schema::Response>,
    },
    /// `update` — streaming.
    Update {
        request: crate::cli::command::update::Request,
        agent_arguments: AgentArguments,
        response: ResponseItemStream<crate::cli::command::update::ResponseItem>,
    },
    /// `update request_schema` — unary.
    UpdateRequestSchema {
        request: crate::cli::command::update::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::update::request_schema::Response>,
    },
    /// `update response_schema` — unary.
    UpdateResponseSchema {
        request: crate::cli::command::update::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::update::response_schema::Response>,
    },
    /// `viewer config address get` — unary.
    ViewerConfigAddressGet {
        request: crate::cli::command::viewer::config::address::get::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::viewer::config::address::get::Response>,
    },
    /// `viewer config address get request_schema` — unary.
    ViewerConfigAddressGetRequestSchema {
        request: crate::cli::command::viewer::config::address::get::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::viewer::config::address::get::request_schema::Response>,
    },
    /// `viewer config address get response_schema` — unary.
    ViewerConfigAddressGetResponseSchema {
        request: crate::cli::command::viewer::config::address::get::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::viewer::config::address::get::response_schema::Response>,
    },
    /// `viewer config address set` — unary.
    ViewerConfigAddressSet {
        request: crate::cli::command::viewer::config::address::set::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::viewer::config::address::set::Response>,
    },
    /// `viewer config address set request_schema` — unary.
    ViewerConfigAddressSetRequestSchema {
        request: crate::cli::command::viewer::config::address::set::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::viewer::config::address::set::request_schema::Response>,
    },
    /// `viewer config address set response_schema` — unary.
    ViewerConfigAddressSetResponseSchema {
        request: crate::cli::command::viewer::config::address::set::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::viewer::config::address::set::response_schema::Response>,
    },
    /// `viewer config get` — unary.
    ViewerConfigGet {
        request: crate::cli::command::viewer::config::get::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::viewer::config::get::Response>,
    },
    /// `viewer config get request_schema` — unary.
    ViewerConfigGetRequestSchema {
        request: crate::cli::command::viewer::config::get::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::viewer::config::get::request_schema::Response>,
    },
    /// `viewer config get response_schema` — unary.
    ViewerConfigGetResponseSchema {
        request: crate::cli::command::viewer::config::get::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::viewer::config::get::response_schema::Response>,
    },
    /// `viewer config secret get` — unary.
    ViewerConfigSecretGet {
        request: crate::cli::command::viewer::config::secret::get::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::viewer::config::secret::get::Response>,
    },
    /// `viewer config secret get request_schema` — unary.
    ViewerConfigSecretGetRequestSchema {
        request: crate::cli::command::viewer::config::secret::get::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::viewer::config::secret::get::request_schema::Response>,
    },
    /// `viewer config secret get response_schema` — unary.
    ViewerConfigSecretGetResponseSchema {
        request: crate::cli::command::viewer::config::secret::get::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::viewer::config::secret::get::response_schema::Response>,
    },
    /// `viewer config secret set` — unary.
    ViewerConfigSecretSet {
        request: crate::cli::command::viewer::config::secret::set::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::viewer::config::secret::set::Response>,
    },
    /// `viewer config secret set request_schema` — unary.
    ViewerConfigSecretSetRequestSchema {
        request: crate::cli::command::viewer::config::secret::set::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::viewer::config::secret::set::request_schema::Response>,
    },
    /// `viewer config secret set response_schema` — unary.
    ViewerConfigSecretSetResponseSchema {
        request: crate::cli::command::viewer::config::secret::set::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::viewer::config::secret::set::response_schema::Response>,
    },
    /// `viewer config signature get` — unary.
    ViewerConfigSignatureGet {
        request: crate::cli::command::viewer::config::signature::get::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::viewer::config::signature::get::Response>,
    },
    /// `viewer config signature get request_schema` — unary.
    ViewerConfigSignatureGetRequestSchema {
        request: crate::cli::command::viewer::config::signature::get::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::viewer::config::signature::get::request_schema::Response>,
    },
    /// `viewer config signature get response_schema` — unary.
    ViewerConfigSignatureGetResponseSchema {
        request: crate::cli::command::viewer::config::signature::get::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::viewer::config::signature::get::response_schema::Response>,
    },
    /// `viewer config signature set` — unary.
    ViewerConfigSignatureSet {
        request: crate::cli::command::viewer::config::signature::set::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::viewer::config::signature::set::Response>,
    },
    /// `viewer config signature set request_schema` — unary.
    ViewerConfigSignatureSetRequestSchema {
        request: crate::cli::command::viewer::config::signature::set::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::viewer::config::signature::set::request_schema::Response>,
    },
    /// `viewer config signature set response_schema` — unary.
    ViewerConfigSignatureSetResponseSchema {
        request: crate::cli::command::viewer::config::signature::set::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::viewer::config::signature::set::response_schema::Response>,
    },
    /// `viewer generate_secret_signature_pair` — unary.
    ViewerGenerateSecretSignaturePair {
        request: crate::cli::command::viewer::generate_secret_signature_pair::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::viewer::generate_secret_signature_pair::Response>,
    },
    /// `viewer generate_secret_signature_pair request_schema` — unary.
    ViewerGenerateSecretSignaturePairRequestSchema {
        request: crate::cli::command::viewer::generate_secret_signature_pair::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::viewer::generate_secret_signature_pair::request_schema::Response>,
    },
    /// `viewer generate_secret_signature_pair response_schema` — unary.
    ViewerGenerateSecretSignaturePairResponseSchema {
        request: crate::cli::command::viewer::generate_secret_signature_pair::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::viewer::generate_secret_signature_pair::response_schema::Response>,
    },
    /// `viewer kill` — unary.
    ViewerKill {
        request: crate::cli::command::viewer::kill::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::viewer::kill::Response>,
    },
    /// `viewer kill request_schema` — unary.
    ViewerKillRequestSchema {
        request: crate::cli::command::viewer::kill::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::viewer::kill::request_schema::Response>,
    },
    /// `viewer kill response_schema` — unary.
    ViewerKillResponseSchema {
        request: crate::cli::command::viewer::kill::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::viewer::kill::response_schema::Response>,
    },
    /// `viewer spawn` — unary.
    ViewerSpawn {
        request: crate::cli::command::viewer::spawn::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::viewer::spawn::Response>,
    },
    /// `viewer spawn request_schema` — unary.
    ViewerSpawnRequestSchema {
        request: crate::cli::command::viewer::spawn::request_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::viewer::spawn::request_schema::Response>,
    },
    /// `viewer spawn response_schema` — unary.
    ViewerSpawnResponseSchema {
        request: crate::cli::command::viewer::spawn::response_schema::Request,
        agent_arguments: AgentArguments,
        response: UnaryResponse<crate::cli::command::viewer::spawn::response_schema::Response>,
    },
    /// The request carries a jq/python output transform: its teed
    /// items are post-transform JSON with no static shape, so they
    /// stay raw wire text — the consumer deserializes each however
    /// it sees fit (the request itself still parses as the typed
    /// root aggregate).
    Transformed {
        request: Box<crate::cli::command::Request>,
        agent_arguments: AgentArguments,
        response: ResponseItemStream<Box<RawValue>>,
    },
}

/// One run's feed side, held by the listener pump keyed by broadcast
/// id: response frames' raw bodies go through [`RunFeed::push`],
/// which deserializes each as the run's exact item type into the
/// typed channel captured inside; [`RunFeed::close`] (the
/// terminator, or connection end) drops the captured sender — which
/// ends the stream, or settles an unresolved unary future with the
/// synthesized ended-without-response error.
pub(crate) struct RunFeed {
    push_fn: Box<dyn FnMut(&RawValue) + Send>,
}

impl RunFeed {
    pub(crate) fn push(&mut self, value: &RawValue) {
        (self.push_fn)(value)
    }

    /// Dropping `self` drops the captured sender — that IS the close.
    pub(crate) fn close(self) {}
}

/// A unary run's response future + feed. The FIRST pushed frame
/// resolves the future (error-first decode); later frames are no-ops.
fn unary_feed<T: serde::de::DeserializeOwned + Send + 'static>(
    path_type: &str,
) -> (UnaryResponse<T>, RunFeed) {
    let (response, tx) = UnaryResponse::channel(path_type.to_string());
    let mut tx = Some(tx);
    let push_fn = Box::new(move |value: &RawValue| {
        if let Some(tx) = tx.take() {
            let _ = tx.send(decode_item::<T>(value));
        }
    });
    (response, RunFeed { push_fn })
}

/// A streaming run's response stream + feed: every pushed frame
/// becomes one item (error-first decode).
fn stream_feed<T: serde::de::DeserializeOwned + Send + 'static>()
-> (ResponseItemStream<T>, RunFeed) {
    let (response, tx) = ResponseItemStream::channel();
    let push_fn = Box::new(move |value: &RawValue| {
        let _ = tx.send(decode_item::<T>(value));
    });
    (response, RunFeed { push_fn })
}

/// The dispatch probe: the only three request fields the dispatch
/// needs before it knows the exact type to deserialize the whole
/// request as. Not a wire schema — a borrowed peek at one.
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(serde::Deserialize)]
struct RequestProbe {
    #[serde(default)]
    path_type: Option<String>,
    #[serde(default)]
    jq: Option<String>,
    #[serde(default)]
    python: Option<String>,
}

/// Open a run from its broadcast request frame's raw body:
/// discriminate via [`RequestProbe`], deserialize the request
/// straight from the wire text as its exact type, and hand back the
/// matching [`Run`] variant plus the feed its response frames go
/// through. Transform-bearing requests (jq/python set) land on
/// [`Run::Transformed`] regardless of leaf — their items are
/// post-transform JSON. `None` — an unrecognized `path_type` or a
/// request that fails to deserialize — means the run is skipped
/// entirely.
pub(crate) fn open_run(
    request: &RawValue,
    agent_arguments: AgentArguments,
) -> Option<(Run, RunFeed)> {
    let probe = serde_json::from_str::<RequestProbe>(request.get()).ok()?;
    if probe.jq.is_some() || probe.python.is_some() {
        let parsed = serde_json::from_str::<crate::cli::command::Request>(request.get()).ok()?;
        let (response, feed) = stream_feed::<Box<RawValue>>();
        return Some((
            Run::Transformed {
                request: Box::new(parsed),
                agent_arguments,
                response,
            },
            feed,
        ));
    }
    let path_type = probe.path_type.as_deref().unwrap_or("");
    match path_type {
        "agents/enqueue" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::enqueue::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::enqueue::Response>(path_type);
            Some((Run::AgentsEnqueue { request: parsed, agent_arguments, response }, feed))
        }
        "agents/enqueue/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::enqueue::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::enqueue::request_schema::Response>(path_type);
            Some((Run::AgentsEnqueueRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/enqueue/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::enqueue::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::enqueue::response_schema::Response>(path_type);
            Some((Run::AgentsEnqueueResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::get::Response>(path_type);
            Some((Run::AgentsGet { request: parsed, agent_arguments, response }, feed))
        }
        "agents/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::get::request_schema::Response>(path_type);
            Some((Run::AgentsGetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::get::response_schema::Response>(path_type);
            Some((Run::AgentsGetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/instances/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::instances::get::Request>(request.get()).ok()?;
            let (response, feed) = stream_feed::<crate::cli::command::agents::instances::get::ResponseItem>();
            Some((Run::AgentsInstancesGet { request: parsed, agent_arguments, response }, feed))
        }
        "agents/instances/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::instances::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::instances::get::request_schema::Response>(path_type);
            Some((Run::AgentsInstancesGetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/instances/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::instances::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::instances::get::response_schema::Response>(path_type);
            Some((Run::AgentsInstancesGetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/instances/list" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::instances::list::Request>(request.get()).ok()?;
            let (response, feed) = stream_feed::<crate::cli::command::agents::instances::list::ResponseItem>();
            Some((Run::AgentsInstancesList { request: parsed, agent_arguments, response }, feed))
        }
        "agents/instances/list/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::instances::list::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::instances::list::request_schema::Response>(path_type);
            Some((Run::AgentsInstancesListRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/instances/list/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::instances::list::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::instances::list::response_schema::Response>(path_type);
            Some((Run::AgentsInstancesListResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/laboratories/attach" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::laboratories::attach::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::laboratories::attach::Response>(path_type);
            Some((Run::AgentsLaboratoriesAttach { request: parsed, agent_arguments, response }, feed))
        }
        "agents/laboratories/attach/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::laboratories::attach::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::laboratories::attach::request_schema::Response>(path_type);
            Some((Run::AgentsLaboratoriesAttachRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/laboratories/attach/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::laboratories::attach::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::laboratories::attach::response_schema::Response>(path_type);
            Some((Run::AgentsLaboratoriesAttachResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/laboratories/detach" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::laboratories::detach::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::laboratories::detach::Response>(path_type);
            Some((Run::AgentsLaboratoriesDetach { request: parsed, agent_arguments, response }, feed))
        }
        "agents/laboratories/detach/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::laboratories::detach::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::laboratories::detach::request_schema::Response>(path_type);
            Some((Run::AgentsLaboratoriesDetachRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/laboratories/detach/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::laboratories::detach::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::laboratories::detach::response_schema::Response>(path_type);
            Some((Run::AgentsLaboratoriesDetachResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/laboratories/list" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::laboratories::list::Request>(request.get()).ok()?;
            let (response, feed) = stream_feed::<crate::cli::command::agents::laboratories::list::ResponseItem>();
            Some((Run::AgentsLaboratoriesList { request: parsed, agent_arguments, response }, feed))
        }
        "agents/laboratories/list/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::laboratories::list::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::laboratories::list::request_schema::Response>(path_type);
            Some((Run::AgentsLaboratoriesListRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/laboratories/list/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::laboratories::list::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::laboratories::list::response_schema::Response>(path_type);
            Some((Run::AgentsLaboratoriesListResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/list" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::list::Request>(request.get()).ok()?;
            let (response, feed) = stream_feed::<crate::cli::command::agents::list::ResponseItem>();
            Some((Run::AgentsList { request: parsed, agent_arguments, response }, feed))
        }
        "agents/list/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::list::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::list::request_schema::Response>(path_type);
            Some((Run::AgentsListRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/list/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::list::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::list::response_schema::Response>(path_type);
            Some((Run::AgentsListResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/logs/list" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::logs::list::Request>(request.get()).ok()?;
            let (response, feed) = stream_feed::<crate::cli::command::agents::logs::list::ResponseItem>();
            Some((Run::AgentsLogsList { request: parsed, agent_arguments, response }, feed))
        }
        "agents/logs/list/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::logs::list::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::logs::list::request_schema::Response>(path_type);
            Some((Run::AgentsLogsListRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/logs/list/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::logs::list::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::logs::list::response_schema::Response>(path_type);
            Some((Run::AgentsLogsListResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/logs/open" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::logs::open::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::logs::open::Response>(path_type);
            Some((Run::AgentsLogsOpen { request: parsed, agent_arguments, response }, feed))
        }
        "agents/logs/open/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::logs::open::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::logs::open::request_schema::Response>(path_type);
            Some((Run::AgentsLogsOpenRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/logs/open/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::logs::open::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::logs::open::response_schema::Response>(path_type);
            Some((Run::AgentsLogsOpenResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/logs/subscribe" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::logs::subscribe::Request>(request.get()).ok()?;
            let (response, feed) = stream_feed::<crate::cli::command::agents::logs::subscribe::ResponseItem>();
            Some((Run::AgentsLogsSubscribe { request: parsed, agent_arguments, response }, feed))
        }
        "agents/logs/subscribe/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::logs::subscribe::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::logs::subscribe::request_schema::Response>(path_type);
            Some((Run::AgentsLogsSubscribeRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/logs/subscribe/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::logs::subscribe::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::logs::subscribe::response_schema::Response>(path_type);
            Some((Run::AgentsLogsSubscribeResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/logs/token-usage/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::logs::token_usage::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::logs::token_usage::get::Response>(path_type);
            Some((Run::AgentsLogsTokenUsageGet { request: parsed, agent_arguments, response }, feed))
        }
        "agents/logs/token-usage/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::logs::token_usage::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::logs::token_usage::get::request_schema::Response>(path_type);
            Some((Run::AgentsLogsTokenUsageGetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/logs/token-usage/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::logs::token_usage::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::logs::token_usage::get::response_schema::Response>(path_type);
            Some((Run::AgentsLogsTokenUsageGetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/logs/token-usage/subscribe" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::logs::token_usage::subscribe::Request>(request.get()).ok()?;
            let (response, feed) = stream_feed::<crate::cli::command::agents::logs::token_usage::subscribe::ResponseItem>();
            Some((Run::AgentsLogsTokenUsageSubscribe { request: parsed, agent_arguments, response }, feed))
        }
        "agents/logs/token-usage/subscribe/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::logs::token_usage::subscribe::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::logs::token_usage::subscribe::request_schema::Response>(path_type);
            Some((Run::AgentsLogsTokenUsageSubscribeRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/logs/token-usage/subscribe/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::logs::token_usage::subscribe::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::logs::token_usage::subscribe::response_schema::Response>(path_type);
            Some((Run::AgentsLogsTokenUsageSubscribeResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/mcp/resources/list" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::mcp::resources::list::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::mcp::resources::list::Response>(path_type);
            Some((Run::AgentsMcpResourcesList { request: parsed, agent_arguments, response }, feed))
        }
        "agents/mcp/resources/list/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::mcp::resources::list::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::mcp::resources::list::request_schema::Response>(path_type);
            Some((Run::AgentsMcpResourcesListRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/mcp/resources/list/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::mcp::resources::list::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::mcp::resources::list::response_schema::Response>(path_type);
            Some((Run::AgentsMcpResourcesListResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/mcp/resources/read" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::mcp::resources::read::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::mcp::resources::read::Response>(path_type);
            Some((Run::AgentsMcpResourcesRead { request: parsed, agent_arguments, response }, feed))
        }
        "agents/mcp/resources/read/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::mcp::resources::read::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::mcp::resources::read::request_schema::Response>(path_type);
            Some((Run::AgentsMcpResourcesReadRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/mcp/resources/read/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::mcp::resources::read::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::mcp::resources::read::response_schema::Response>(path_type);
            Some((Run::AgentsMcpResourcesReadResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/mcp/servers/list" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::mcp::servers::list::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::mcp::servers::list::Response>(path_type);
            Some((Run::AgentsMcpServersList { request: parsed, agent_arguments, response }, feed))
        }
        "agents/mcp/servers/list/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::mcp::servers::list::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::mcp::servers::list::request_schema::Response>(path_type);
            Some((Run::AgentsMcpServersListRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/mcp/servers/list/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::mcp::servers::list::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::mcp::servers::list::response_schema::Response>(path_type);
            Some((Run::AgentsMcpServersListResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/mcp/tools/call" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::mcp::tools::call::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::mcp::tools::call::Response>(path_type);
            Some((Run::AgentsMcpToolsCall { request: parsed, agent_arguments, response }, feed))
        }
        "agents/mcp/tools/call/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::mcp::tools::call::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::mcp::tools::call::request_schema::Response>(path_type);
            Some((Run::AgentsMcpToolsCallRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/mcp/tools/call/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::mcp::tools::call::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::mcp::tools::call::response_schema::Response>(path_type);
            Some((Run::AgentsMcpToolsCallResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/mcp/tools/list" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::mcp::tools::list::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::mcp::tools::list::Response>(path_type);
            Some((Run::AgentsMcpToolsList { request: parsed, agent_arguments, response }, feed))
        }
        "agents/mcp/tools/list/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::mcp::tools::list::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::mcp::tools::list::request_schema::Response>(path_type);
            Some((Run::AgentsMcpToolsListRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/mcp/tools/list/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::mcp::tools::list::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::mcp::tools::list::response_schema::Response>(path_type);
            Some((Run::AgentsMcpToolsListResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/message" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::message::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::message::Response>(path_type);
            Some((Run::AgentsMessage { request: parsed, agent_arguments, response }, feed))
        }
        "agents/message/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::message::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::message::request_schema::Response>(path_type);
            Some((Run::AgentsMessageRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/message/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::message::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::message::response_schema::Response>(path_type);
            Some((Run::AgentsMessageResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/publish" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::publish::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::publish::Response>(path_type);
            Some((Run::AgentsPublish { request: parsed, agent_arguments, response }, feed))
        }
        "agents/publish/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::publish::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::publish::request_schema::Response>(path_type);
            Some((Run::AgentsPublishRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/publish/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::publish::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::publish::response_schema::Response>(path_type);
            Some((Run::AgentsPublishResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/queue/delete" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::queue::delete::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::queue::delete::Response>(path_type);
            Some((Run::AgentsQueueDelete { request: parsed, agent_arguments, response }, feed))
        }
        "agents/queue/delete/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::queue::delete::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::queue::delete::request_schema::Response>(path_type);
            Some((Run::AgentsQueueDeleteRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/queue/delete/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::queue::delete::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::queue::delete::response_schema::Response>(path_type);
            Some((Run::AgentsQueueDeleteResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/queue/deliver" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::queue::deliver::Request>(request.get()).ok()?;
            let (response, feed) = stream_feed::<crate::cli::command::agents::queue::deliver::ResponseItem>();
            Some((Run::AgentsQueueDeliver { request: parsed, agent_arguments, response }, feed))
        }
        "agents/queue/deliver/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::queue::deliver::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::queue::deliver::request_schema::Response>(path_type);
            Some((Run::AgentsQueueDeliverRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/queue/deliver/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::queue::deliver::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::queue::deliver::response_schema::Response>(path_type);
            Some((Run::AgentsQueueDeliverResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/queue/list" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::queue::list::Request>(request.get()).ok()?;
            let (response, feed) = stream_feed::<crate::cli::command::agents::queue::list::ResponseItem>();
            Some((Run::AgentsQueueList { request: parsed, agent_arguments, response }, feed))
        }
        "agents/queue/list/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::queue::list::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::queue::list::request_schema::Response>(path_type);
            Some((Run::AgentsQueueListRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/queue/list/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::queue::list::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::queue::list::response_schema::Response>(path_type);
            Some((Run::AgentsQueueListResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/queue/open" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::queue::open::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::queue::open::Response>(path_type);
            Some((Run::AgentsQueueOpen { request: parsed, agent_arguments, response }, feed))
        }
        "agents/queue/open/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::queue::open::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::queue::open::request_schema::Response>(path_type);
            Some((Run::AgentsQueueOpenRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/queue/open/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::queue::open::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::queue::open::response_schema::Response>(path_type);
            Some((Run::AgentsQueueOpenResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/spawn" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::spawn::Request>(request.get()).ok()?;
            if parsed.dangerous_advanced.as_ref().and_then(|a| a.stream) == Some(true) {
                let (response, feed) = stream_feed::<crate::cli::command::agents::spawn::ResponseItem>();
                Some((Run::AgentsSpawnStreaming { request: parsed, agent_arguments, response }, feed))
            } else {
                let (response, feed) = unary_feed::<crate::cli::command::agents::spawn::Response>(path_type);
                Some((Run::AgentsSpawn { request: parsed, agent_arguments, response }, feed))
            }
        }
        "agents/spawn/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::spawn::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::spawn::request_schema::Response>(path_type);
            Some((Run::AgentsSpawnRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/spawn/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::spawn::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::spawn::response_schema::Response>(path_type);
            Some((Run::AgentsSpawnResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/tags/apply" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::tags::apply::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::tags::apply::Response>(path_type);
            Some((Run::AgentsTagsApply { request: parsed, agent_arguments, response }, feed))
        }
        "agents/tags/apply/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::tags::apply::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::tags::apply::request_schema::Response>(path_type);
            Some((Run::AgentsTagsApplyRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/tags/apply/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::tags::apply::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::tags::apply::response_schema::Response>(path_type);
            Some((Run::AgentsTagsApplyResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/tags/lookup/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::tags::lookup::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::tags::lookup::request_schema::Response>(path_type);
            Some((Run::AgentsTagsLookupRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/tags/lookup/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::tags::lookup::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::tags::lookup::response_schema::Response>(path_type);
            Some((Run::AgentsTagsLookupResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/wait" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::wait::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::wait::Response>(path_type);
            Some((Run::AgentsWait { request: parsed, agent_arguments, response }, feed))
        }
        "agents/wait/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::wait::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::wait::request_schema::Response>(path_type);
            Some((Run::AgentsWaitRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "agents/wait/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::wait::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::wait::response_schema::Response>(path_type);
            Some((Run::AgentsWaitResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/address/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::address::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::address::get::Response>(path_type);
            Some((Run::ApiConfigAddressGet { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/address/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::address::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::address::get::request_schema::Response>(path_type);
            Some((Run::ApiConfigAddressGetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/address/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::address::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::address::get::response_schema::Response>(path_type);
            Some((Run::ApiConfigAddressGetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/address/set" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::address::set::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::address::set::Response>(path_type);
            Some((Run::ApiConfigAddressSet { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/address/set/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::address::set::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::address::set::request_schema::Response>(path_type);
            Some((Run::ApiConfigAddressSetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/address/set/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::address::set::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::address::set::response_schema::Response>(path_type);
            Some((Run::ApiConfigAddressSetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/backoff_max_elapsed_time_ms/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::backoff_max_elapsed_time_ms::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::backoff_max_elapsed_time_ms::get::Response>(path_type);
            Some((Run::ApiConfigBackoffMaxElapsedTimeMsGet { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/backoff_max_elapsed_time_ms/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::backoff_max_elapsed_time_ms::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::backoff_max_elapsed_time_ms::get::request_schema::Response>(path_type);
            Some((Run::ApiConfigBackoffMaxElapsedTimeMsGetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/backoff_max_elapsed_time_ms/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::backoff_max_elapsed_time_ms::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::backoff_max_elapsed_time_ms::get::response_schema::Response>(path_type);
            Some((Run::ApiConfigBackoffMaxElapsedTimeMsGetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/backoff_max_elapsed_time_ms/set" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::backoff_max_elapsed_time_ms::set::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::backoff_max_elapsed_time_ms::set::Response>(path_type);
            Some((Run::ApiConfigBackoffMaxElapsedTimeMsSet { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/backoff_max_elapsed_time_ms/set/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::backoff_max_elapsed_time_ms::set::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::backoff_max_elapsed_time_ms::set::request_schema::Response>(path_type);
            Some((Run::ApiConfigBackoffMaxElapsedTimeMsSetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/backoff_max_elapsed_time_ms/set/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::backoff_max_elapsed_time_ms::set::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::backoff_max_elapsed_time_ms::set::response_schema::Response>(path_type);
            Some((Run::ApiConfigBackoffMaxElapsedTimeMsSetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/commit_author_email/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::commit_author_email::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::commit_author_email::get::Response>(path_type);
            Some((Run::ApiConfigCommitAuthorEmailGet { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/commit_author_email/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::commit_author_email::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::commit_author_email::get::request_schema::Response>(path_type);
            Some((Run::ApiConfigCommitAuthorEmailGetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/commit_author_email/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::commit_author_email::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::commit_author_email::get::response_schema::Response>(path_type);
            Some((Run::ApiConfigCommitAuthorEmailGetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/commit_author_email/set" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::commit_author_email::set::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::commit_author_email::set::Response>(path_type);
            Some((Run::ApiConfigCommitAuthorEmailSet { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/commit_author_email/set/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::commit_author_email::set::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::commit_author_email::set::request_schema::Response>(path_type);
            Some((Run::ApiConfigCommitAuthorEmailSetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/commit_author_email/set/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::commit_author_email::set::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::commit_author_email::set::response_schema::Response>(path_type);
            Some((Run::ApiConfigCommitAuthorEmailSetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/commit_author_name/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::commit_author_name::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::commit_author_name::get::Response>(path_type);
            Some((Run::ApiConfigCommitAuthorNameGet { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/commit_author_name/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::commit_author_name::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::commit_author_name::get::request_schema::Response>(path_type);
            Some((Run::ApiConfigCommitAuthorNameGetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/commit_author_name/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::commit_author_name::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::commit_author_name::get::response_schema::Response>(path_type);
            Some((Run::ApiConfigCommitAuthorNameGetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/commit_author_name/set" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::commit_author_name::set::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::commit_author_name::set::Response>(path_type);
            Some((Run::ApiConfigCommitAuthorNameSet { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/commit_author_name/set/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::commit_author_name::set::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::commit_author_name::set::request_schema::Response>(path_type);
            Some((Run::ApiConfigCommitAuthorNameSetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/commit_author_name/set/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::commit_author_name::set::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::commit_author_name::set::response_schema::Response>(path_type);
            Some((Run::ApiConfigCommitAuthorNameSetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::get::Response>(path_type);
            Some((Run::ApiConfigGet { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::get::request_schema::Response>(path_type);
            Some((Run::ApiConfigGetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::get::response_schema::Response>(path_type);
            Some((Run::ApiConfigGetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/github_authorization/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::github_authorization::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::github_authorization::get::Response>(path_type);
            Some((Run::ApiConfigGithubAuthorizationGet { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/github_authorization/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::github_authorization::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::github_authorization::get::request_schema::Response>(path_type);
            Some((Run::ApiConfigGithubAuthorizationGetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/github_authorization/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::github_authorization::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::github_authorization::get::response_schema::Response>(path_type);
            Some((Run::ApiConfigGithubAuthorizationGetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/github_authorization/set" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::github_authorization::set::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::github_authorization::set::Response>(path_type);
            Some((Run::ApiConfigGithubAuthorizationSet { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/github_authorization/set/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::github_authorization::set::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::github_authorization::set::request_schema::Response>(path_type);
            Some((Run::ApiConfigGithubAuthorizationSetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/github_authorization/set/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::github_authorization::set::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::github_authorization::set::response_schema::Response>(path_type);
            Some((Run::ApiConfigGithubAuthorizationSetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/http_referer/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::http_referer::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::http_referer::get::Response>(path_type);
            Some((Run::ApiConfigHttpRefererGet { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/http_referer/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::http_referer::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::http_referer::get::request_schema::Response>(path_type);
            Some((Run::ApiConfigHttpRefererGetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/http_referer/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::http_referer::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::http_referer::get::response_schema::Response>(path_type);
            Some((Run::ApiConfigHttpRefererGetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/http_referer/set" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::http_referer::set::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::http_referer::set::Response>(path_type);
            Some((Run::ApiConfigHttpRefererSet { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/http_referer/set/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::http_referer::set::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::http_referer::set::request_schema::Response>(path_type);
            Some((Run::ApiConfigHttpRefererSetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/http_referer/set/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::http_referer::set::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::http_referer::set::response_schema::Response>(path_type);
            Some((Run::ApiConfigHttpRefererSetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/mcp_authorization/add" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::mcp_authorization::add::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::mcp_authorization::add::Response>(path_type);
            Some((Run::ApiConfigMcpAuthorizationAdd { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/mcp_authorization/add/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::mcp_authorization::add::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::mcp_authorization::add::request_schema::Response>(path_type);
            Some((Run::ApiConfigMcpAuthorizationAddRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/mcp_authorization/add/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::mcp_authorization::add::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::mcp_authorization::add::response_schema::Response>(path_type);
            Some((Run::ApiConfigMcpAuthorizationAddResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/mcp_authorization/del" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::mcp_authorization::del::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::mcp_authorization::del::Response>(path_type);
            Some((Run::ApiConfigMcpAuthorizationDel { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/mcp_authorization/del/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::mcp_authorization::del::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::mcp_authorization::del::request_schema::Response>(path_type);
            Some((Run::ApiConfigMcpAuthorizationDelRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/mcp_authorization/del/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::mcp_authorization::del::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::mcp_authorization::del::response_schema::Response>(path_type);
            Some((Run::ApiConfigMcpAuthorizationDelResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/mcp_authorization/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::mcp_authorization::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::mcp_authorization::get::Response>(path_type);
            Some((Run::ApiConfigMcpAuthorizationGet { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/mcp_authorization/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::mcp_authorization::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::mcp_authorization::get::request_schema::Response>(path_type);
            Some((Run::ApiConfigMcpAuthorizationGetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/mcp_authorization/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::mcp_authorization::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::mcp_authorization::get::response_schema::Response>(path_type);
            Some((Run::ApiConfigMcpAuthorizationGetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/mcp_timeout_ms/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::mcp_timeout_ms::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::mcp_timeout_ms::get::Response>(path_type);
            Some((Run::ApiConfigMcpTimeoutMsGet { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/mcp_timeout_ms/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::mcp_timeout_ms::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::mcp_timeout_ms::get::request_schema::Response>(path_type);
            Some((Run::ApiConfigMcpTimeoutMsGetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/mcp_timeout_ms/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::mcp_timeout_ms::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::mcp_timeout_ms::get::response_schema::Response>(path_type);
            Some((Run::ApiConfigMcpTimeoutMsGetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/mcp_timeout_ms/set" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::mcp_timeout_ms::set::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::mcp_timeout_ms::set::Response>(path_type);
            Some((Run::ApiConfigMcpTimeoutMsSet { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/mcp_timeout_ms/set/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::mcp_timeout_ms::set::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::mcp_timeout_ms::set::request_schema::Response>(path_type);
            Some((Run::ApiConfigMcpTimeoutMsSetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/mcp_timeout_ms/set/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::mcp_timeout_ms::set::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::mcp_timeout_ms::set::response_schema::Response>(path_type);
            Some((Run::ApiConfigMcpTimeoutMsSetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/objectiveai_authorization/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::objectiveai_authorization::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::objectiveai_authorization::get::Response>(path_type);
            Some((Run::ApiConfigObjectiveaiAuthorizationGet { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/objectiveai_authorization/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::objectiveai_authorization::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::objectiveai_authorization::get::request_schema::Response>(path_type);
            Some((Run::ApiConfigObjectiveaiAuthorizationGetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/objectiveai_authorization/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::objectiveai_authorization::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::objectiveai_authorization::get::response_schema::Response>(path_type);
            Some((Run::ApiConfigObjectiveaiAuthorizationGetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/objectiveai_authorization/set" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::objectiveai_authorization::set::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::objectiveai_authorization::set::Response>(path_type);
            Some((Run::ApiConfigObjectiveaiAuthorizationSet { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/objectiveai_authorization/set/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::objectiveai_authorization::set::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::objectiveai_authorization::set::request_schema::Response>(path_type);
            Some((Run::ApiConfigObjectiveaiAuthorizationSetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/objectiveai_authorization/set/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::objectiveai_authorization::set::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::objectiveai_authorization::set::response_schema::Response>(path_type);
            Some((Run::ApiConfigObjectiveaiAuthorizationSetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/openrouter_authorization/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::openrouter_authorization::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::openrouter_authorization::get::Response>(path_type);
            Some((Run::ApiConfigOpenrouterAuthorizationGet { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/openrouter_authorization/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::openrouter_authorization::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::openrouter_authorization::get::request_schema::Response>(path_type);
            Some((Run::ApiConfigOpenrouterAuthorizationGetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/openrouter_authorization/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::openrouter_authorization::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::openrouter_authorization::get::response_schema::Response>(path_type);
            Some((Run::ApiConfigOpenrouterAuthorizationGetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/openrouter_authorization/set" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::openrouter_authorization::set::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::openrouter_authorization::set::Response>(path_type);
            Some((Run::ApiConfigOpenrouterAuthorizationSet { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/openrouter_authorization/set/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::openrouter_authorization::set::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::openrouter_authorization::set::request_schema::Response>(path_type);
            Some((Run::ApiConfigOpenrouterAuthorizationSetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/openrouter_authorization/set/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::openrouter_authorization::set::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::openrouter_authorization::set::response_schema::Response>(path_type);
            Some((Run::ApiConfigOpenrouterAuthorizationSetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/user_agent/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::user_agent::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::user_agent::get::Response>(path_type);
            Some((Run::ApiConfigUserAgentGet { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/user_agent/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::user_agent::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::user_agent::get::request_schema::Response>(path_type);
            Some((Run::ApiConfigUserAgentGetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/user_agent/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::user_agent::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::user_agent::get::response_schema::Response>(path_type);
            Some((Run::ApiConfigUserAgentGetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/user_agent/set" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::user_agent::set::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::user_agent::set::Response>(path_type);
            Some((Run::ApiConfigUserAgentSet { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/user_agent/set/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::user_agent::set::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::user_agent::set::request_schema::Response>(path_type);
            Some((Run::ApiConfigUserAgentSetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/user_agent/set/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::user_agent::set::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::user_agent::set::response_schema::Response>(path_type);
            Some((Run::ApiConfigUserAgentSetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/x_title/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::x_title::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::x_title::get::Response>(path_type);
            Some((Run::ApiConfigXTitleGet { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/x_title/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::x_title::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::x_title::get::request_schema::Response>(path_type);
            Some((Run::ApiConfigXTitleGetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/x_title/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::x_title::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::x_title::get::response_schema::Response>(path_type);
            Some((Run::ApiConfigXTitleGetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/x_title/set" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::x_title::set::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::x_title::set::Response>(path_type);
            Some((Run::ApiConfigXTitleSet { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/x_title/set/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::x_title::set::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::x_title::set::request_schema::Response>(path_type);
            Some((Run::ApiConfigXTitleSetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/config/x_title/set/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::x_title::set::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::x_title::set::response_schema::Response>(path_type);
            Some((Run::ApiConfigXTitleSetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/kill" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::kill::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::kill::Response>(path_type);
            Some((Run::ApiKill { request: parsed, agent_arguments, response }, feed))
        }
        "api/kill/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::kill::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::kill::request_schema::Response>(path_type);
            Some((Run::ApiKillRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/kill/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::kill::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::kill::response_schema::Response>(path_type);
            Some((Run::ApiKillResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/spawn" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::spawn::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::spawn::Response>(path_type);
            Some((Run::ApiSpawn { request: parsed, agent_arguments, response }, feed))
        }
        "api/spawn/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::spawn::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::spawn::request_schema::Response>(path_type);
            Some((Run::ApiSpawnRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "api/spawn/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::spawn::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::spawn::response_schema::Response>(path_type);
            Some((Run::ApiSpawnResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "daemon/kill" => {
            let parsed = serde_json::from_str::<crate::cli::command::daemon::kill::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::daemon::kill::Response>(path_type);
            Some((Run::DaemonKill { request: parsed, agent_arguments, response }, feed))
        }
        "daemon/kill/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::daemon::kill::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::daemon::kill::request_schema::Response>(path_type);
            Some((Run::DaemonKillRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "daemon/kill/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::daemon::kill::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::daemon::kill::response_schema::Response>(path_type);
            Some((Run::DaemonKillResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "daemon/spawn" => {
            let parsed = serde_json::from_str::<crate::cli::command::daemon::spawn::Request>(request.get()).ok()?;
            let (response, feed) = stream_feed::<crate::cli::command::daemon::spawn::ResponseItem>();
            Some((Run::DaemonSpawn { request: parsed, agent_arguments, response }, feed))
        }
        "daemon/spawn/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::daemon::spawn::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::daemon::spawn::request_schema::Response>(path_type);
            Some((Run::DaemonSpawnRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "daemon/spawn/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::daemon::spawn::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::daemon::spawn::response_schema::Response>(path_type);
            Some((Run::DaemonSpawnResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "db/config/address/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::address::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::address::get::Response>(path_type);
            Some((Run::DbConfigAddressGet { request: parsed, agent_arguments, response }, feed))
        }
        "db/config/address/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::address::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::address::get::request_schema::Response>(path_type);
            Some((Run::DbConfigAddressGetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "db/config/address/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::address::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::address::get::response_schema::Response>(path_type);
            Some((Run::DbConfigAddressGetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "db/config/address/set" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::address::set::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::address::set::Response>(path_type);
            Some((Run::DbConfigAddressSet { request: parsed, agent_arguments, response }, feed))
        }
        "db/config/address/set/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::address::set::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::address::set::request_schema::Response>(path_type);
            Some((Run::DbConfigAddressSetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "db/config/address/set/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::address::set::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::address::set::response_schema::Response>(path_type);
            Some((Run::DbConfigAddressSetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "db/config/database/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::database::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::database::get::Response>(path_type);
            Some((Run::DbConfigDatabaseGet { request: parsed, agent_arguments, response }, feed))
        }
        "db/config/database/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::database::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::database::get::request_schema::Response>(path_type);
            Some((Run::DbConfigDatabaseGetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "db/config/database/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::database::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::database::get::response_schema::Response>(path_type);
            Some((Run::DbConfigDatabaseGetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "db/config/database/set" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::database::set::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::database::set::Response>(path_type);
            Some((Run::DbConfigDatabaseSet { request: parsed, agent_arguments, response }, feed))
        }
        "db/config/database/set/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::database::set::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::database::set::request_schema::Response>(path_type);
            Some((Run::DbConfigDatabaseSetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "db/config/database/set/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::database::set::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::database::set::response_schema::Response>(path_type);
            Some((Run::DbConfigDatabaseSetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "db/config/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::get::Response>(path_type);
            Some((Run::DbConfigGet { request: parsed, agent_arguments, response }, feed))
        }
        "db/config/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::get::request_schema::Response>(path_type);
            Some((Run::DbConfigGetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "db/config/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::get::response_schema::Response>(path_type);
            Some((Run::DbConfigGetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "db/config/password/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::password::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::password::get::Response>(path_type);
            Some((Run::DbConfigPasswordGet { request: parsed, agent_arguments, response }, feed))
        }
        "db/config/password/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::password::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::password::get::request_schema::Response>(path_type);
            Some((Run::DbConfigPasswordGetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "db/config/password/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::password::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::password::get::response_schema::Response>(path_type);
            Some((Run::DbConfigPasswordGetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "db/config/password/set" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::password::set::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::password::set::Response>(path_type);
            Some((Run::DbConfigPasswordSet { request: parsed, agent_arguments, response }, feed))
        }
        "db/config/password/set/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::password::set::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::password::set::request_schema::Response>(path_type);
            Some((Run::DbConfigPasswordSetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "db/config/password/set/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::password::set::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::password::set::response_schema::Response>(path_type);
            Some((Run::DbConfigPasswordSetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "db/config/user/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::user::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::user::get::Response>(path_type);
            Some((Run::DbConfigUserGet { request: parsed, agent_arguments, response }, feed))
        }
        "db/config/user/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::user::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::user::get::request_schema::Response>(path_type);
            Some((Run::DbConfigUserGetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "db/config/user/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::user::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::user::get::response_schema::Response>(path_type);
            Some((Run::DbConfigUserGetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "db/config/user/set" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::user::set::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::user::set::Response>(path_type);
            Some((Run::DbConfigUserSet { request: parsed, agent_arguments, response }, feed))
        }
        "db/config/user/set/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::user::set::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::user::set::request_schema::Response>(path_type);
            Some((Run::DbConfigUserSetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "db/config/user/set/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::user::set::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::user::set::response_schema::Response>(path_type);
            Some((Run::DbConfigUserSetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "db/kill" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::kill::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::kill::Response>(path_type);
            Some((Run::DbKill { request: parsed, agent_arguments, response }, feed))
        }
        "db/kill/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::kill::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::kill::request_schema::Response>(path_type);
            Some((Run::DbKillRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "db/kill/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::kill::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::kill::response_schema::Response>(path_type);
            Some((Run::DbKillResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "db/query" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::query::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::query::Response>(path_type);
            Some((Run::DbQuery { request: parsed, agent_arguments, response }, feed))
        }
        "db/query/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::query::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::query::request_schema::Response>(path_type);
            Some((Run::DbQueryRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "db/query/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::query::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::query::response_schema::Response>(path_type);
            Some((Run::DbQueryResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "db/spawn" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::spawn::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::spawn::Response>(path_type);
            Some((Run::DbSpawn { request: parsed, agent_arguments, response }, feed))
        }
        "db/spawn/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::spawn::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::spawn::request_schema::Response>(path_type);
            Some((Run::DbSpawnRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "db/spawn/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::spawn::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::spawn::response_schema::Response>(path_type);
            Some((Run::DbSpawnResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "functions/execute/standard" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::execute::standard::Request>(request.get()).ok()?;
            if parsed.dangerous_advanced.as_ref().and_then(|a| a.stream) == Some(true) {
                let (response, feed) = stream_feed::<crate::cli::command::functions::execute::standard::ResponseItem>();
                Some((Run::FunctionsExecuteStandardStreaming { request: parsed, agent_arguments, response }, feed))
            } else {
                let (response, feed) = unary_feed::<crate::cli::command::functions::execute::standard::Response>(path_type);
                Some((Run::FunctionsExecuteStandard { request: parsed, agent_arguments, response }, feed))
            }
        }
        "functions/execute/standard/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::execute::standard::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::functions::execute::standard::request_schema::Response>(path_type);
            Some((Run::FunctionsExecuteStandardRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "functions/execute/standard/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::execute::standard::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::functions::execute::standard::response_schema::Response>(path_type);
            Some((Run::FunctionsExecuteStandardResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "functions/execute/swiss_system" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::execute::swiss_system::Request>(request.get()).ok()?;
            if parsed.dangerous_advanced.as_ref().and_then(|a| a.stream) == Some(true) {
                let (response, feed) = stream_feed::<crate::cli::command::functions::execute::swiss_system::ResponseItem>();
                Some((Run::FunctionsExecuteSwissSystemStreaming { request: parsed, agent_arguments, response }, feed))
            } else {
                let (response, feed) = unary_feed::<crate::cli::command::functions::execute::swiss_system::Response>(path_type);
                Some((Run::FunctionsExecuteSwissSystem { request: parsed, agent_arguments, response }, feed))
            }
        }
        "functions/execute/swiss_system/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::execute::swiss_system::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::functions::execute::swiss_system::request_schema::Response>(path_type);
            Some((Run::FunctionsExecuteSwissSystemRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "functions/execute/swiss_system/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::execute::swiss_system::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::functions::execute::swiss_system::response_schema::Response>(path_type);
            Some((Run::FunctionsExecuteSwissSystemResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "functions/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::functions::get::Response>(path_type);
            Some((Run::FunctionsGet { request: parsed, agent_arguments, response }, feed))
        }
        "functions/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::functions::get::request_schema::Response>(path_type);
            Some((Run::FunctionsGetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "functions/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::functions::get::response_schema::Response>(path_type);
            Some((Run::FunctionsGetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "functions/list" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::list::Request>(request.get()).ok()?;
            let (response, feed) = stream_feed::<crate::cli::command::functions::list::ResponseItem>();
            Some((Run::FunctionsList { request: parsed, agent_arguments, response }, feed))
        }
        "functions/list/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::list::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::functions::list::request_schema::Response>(path_type);
            Some((Run::FunctionsListRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "functions/list/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::list::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::functions::list::response_schema::Response>(path_type);
            Some((Run::FunctionsListResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "functions/profiles/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::profiles::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::functions::profiles::get::Response>(path_type);
            Some((Run::FunctionsProfilesGet { request: parsed, agent_arguments, response }, feed))
        }
        "functions/profiles/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::profiles::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::functions::profiles::get::request_schema::Response>(path_type);
            Some((Run::FunctionsProfilesGetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "functions/profiles/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::profiles::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::functions::profiles::get::response_schema::Response>(path_type);
            Some((Run::FunctionsProfilesGetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "functions/profiles/list" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::profiles::list::Request>(request.get()).ok()?;
            let (response, feed) = stream_feed::<crate::cli::command::functions::profiles::list::ResponseItem>();
            Some((Run::FunctionsProfilesList { request: parsed, agent_arguments, response }, feed))
        }
        "functions/profiles/list/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::profiles::list::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::functions::profiles::list::request_schema::Response>(path_type);
            Some((Run::FunctionsProfilesListRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "functions/profiles/list/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::profiles::list::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::functions::profiles::list::response_schema::Response>(path_type);
            Some((Run::FunctionsProfilesListResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "functions/profiles/publish" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::profiles::publish::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::functions::profiles::publish::Response>(path_type);
            Some((Run::FunctionsProfilesPublish { request: parsed, agent_arguments, response }, feed))
        }
        "functions/profiles/publish/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::profiles::publish::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::functions::profiles::publish::request_schema::Response>(path_type);
            Some((Run::FunctionsProfilesPublishRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "functions/profiles/publish/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::profiles::publish::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::functions::profiles::publish::response_schema::Response>(path_type);
            Some((Run::FunctionsProfilesPublishResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "functions/publish" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::publish::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::functions::publish::Response>(path_type);
            Some((Run::FunctionsPublish { request: parsed, agent_arguments, response }, feed))
        }
        "functions/publish/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::publish::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::functions::publish::request_schema::Response>(path_type);
            Some((Run::FunctionsPublishRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "functions/publish/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::publish::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::functions::publish::response_schema::Response>(path_type);
            Some((Run::FunctionsPublishResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "kill-all" => {
            let parsed = serde_json::from_str::<crate::cli::command::kill_all::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::kill_all::Response>(path_type);
            Some((Run::KillAll { request: parsed, agent_arguments, response }, feed))
        }
        "kill-all/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::kill_all::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::kill_all::request_schema::Response>(path_type);
            Some((Run::KillAllRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "kill-all/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::kill_all::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::kill_all::response_schema::Response>(path_type);
            Some((Run::KillAllResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "laboratories/create" => {
            let parsed = serde_json::from_str::<crate::cli::command::laboratories::create::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::laboratories::create::Response>(path_type);
            Some((Run::LaboratoriesCreate { request: parsed, agent_arguments, response }, feed))
        }
        "laboratories/create/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::laboratories::create::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::laboratories::create::request_schema::Response>(path_type);
            Some((Run::LaboratoriesCreateRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "laboratories/create/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::laboratories::create::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::laboratories::create::response_schema::Response>(path_type);
            Some((Run::LaboratoriesCreateResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "laboratories/list" => {
            let parsed = serde_json::from_str::<crate::cli::command::laboratories::list::Request>(request.get()).ok()?;
            let (response, feed) = stream_feed::<crate::cli::command::laboratories::list::ResponseItem>();
            Some((Run::LaboratoriesList { request: parsed, agent_arguments, response }, feed))
        }
        "laboratories/list/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::laboratories::list::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::laboratories::list::request_schema::Response>(path_type);
            Some((Run::LaboratoriesListRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "laboratories/list/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::laboratories::list::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::laboratories::list::response_schema::Response>(path_type);
            Some((Run::LaboratoriesListResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "mcp/config/address/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::mcp::config::address::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::mcp::config::address::get::Response>(path_type);
            Some((Run::McpConfigAddressGet { request: parsed, agent_arguments, response }, feed))
        }
        "mcp/config/address/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::mcp::config::address::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::mcp::config::address::get::request_schema::Response>(path_type);
            Some((Run::McpConfigAddressGetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "mcp/config/address/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::mcp::config::address::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::mcp::config::address::get::response_schema::Response>(path_type);
            Some((Run::McpConfigAddressGetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "mcp/config/address/set" => {
            let parsed = serde_json::from_str::<crate::cli::command::mcp::config::address::set::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::mcp::config::address::set::Response>(path_type);
            Some((Run::McpConfigAddressSet { request: parsed, agent_arguments, response }, feed))
        }
        "mcp/config/address/set/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::mcp::config::address::set::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::mcp::config::address::set::request_schema::Response>(path_type);
            Some((Run::McpConfigAddressSetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "mcp/config/address/set/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::mcp::config::address::set::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::mcp::config::address::set::response_schema::Response>(path_type);
            Some((Run::McpConfigAddressSetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "mcp/config/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::mcp::config::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::mcp::config::get::Response>(path_type);
            Some((Run::McpConfigGet { request: parsed, agent_arguments, response }, feed))
        }
        "mcp/config/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::mcp::config::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::mcp::config::get::request_schema::Response>(path_type);
            Some((Run::McpConfigGetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "mcp/config/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::mcp::config::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::mcp::config::get::response_schema::Response>(path_type);
            Some((Run::McpConfigGetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "mcp/config/port/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::mcp::config::port::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::mcp::config::port::get::Response>(path_type);
            Some((Run::McpConfigPortGet { request: parsed, agent_arguments, response }, feed))
        }
        "mcp/config/port/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::mcp::config::port::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::mcp::config::port::get::request_schema::Response>(path_type);
            Some((Run::McpConfigPortGetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "mcp/config/port/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::mcp::config::port::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::mcp::config::port::get::response_schema::Response>(path_type);
            Some((Run::McpConfigPortGetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "mcp/config/port/set" => {
            let parsed = serde_json::from_str::<crate::cli::command::mcp::config::port::set::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::mcp::config::port::set::Response>(path_type);
            Some((Run::McpConfigPortSet { request: parsed, agent_arguments, response }, feed))
        }
        "mcp/config/port/set/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::mcp::config::port::set::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::mcp::config::port::set::request_schema::Response>(path_type);
            Some((Run::McpConfigPortSetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "mcp/config/port/set/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::mcp::config::port::set::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::mcp::config::port::set::response_schema::Response>(path_type);
            Some((Run::McpConfigPortSetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "mcp/kill" => {
            let parsed = serde_json::from_str::<crate::cli::command::mcp::kill::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::mcp::kill::Response>(path_type);
            Some((Run::McpKill { request: parsed, agent_arguments, response }, feed))
        }
        "mcp/kill/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::mcp::kill::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::mcp::kill::request_schema::Response>(path_type);
            Some((Run::McpKillRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "mcp/kill/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::mcp::kill::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::mcp::kill::response_schema::Response>(path_type);
            Some((Run::McpKillResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "mcp/spawn" => {
            let parsed = serde_json::from_str::<crate::cli::command::mcp::spawn::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::mcp::spawn::Response>(path_type);
            Some((Run::McpSpawn { request: parsed, agent_arguments, response }, feed))
        }
        "mcp/spawn/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::mcp::spawn::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::mcp::spawn::request_schema::Response>(path_type);
            Some((Run::McpSpawnRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "mcp/spawn/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::mcp::spawn::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::mcp::spawn::response_schema::Response>(path_type);
            Some((Run::McpSpawnResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "plugins/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::plugins::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::plugins::get::Response>(path_type);
            Some((Run::PluginsGet { request: parsed, agent_arguments, response }, feed))
        }
        "plugins/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::plugins::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::plugins::get::request_schema::Response>(path_type);
            Some((Run::PluginsGetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "plugins/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::plugins::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::plugins::get::response_schema::Response>(path_type);
            Some((Run::PluginsGetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "plugins/install/filesystem" => {
            let parsed = serde_json::from_str::<crate::cli::command::plugins::install::filesystem::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::plugins::install::filesystem::Response>(path_type);
            Some((Run::PluginsInstallFilesystem { request: parsed, agent_arguments, response }, feed))
        }
        "plugins/install/filesystem/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::plugins::install::filesystem::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::plugins::install::filesystem::request_schema::Response>(path_type);
            Some((Run::PluginsInstallFilesystemRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "plugins/install/filesystem/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::plugins::install::filesystem::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::plugins::install::filesystem::response_schema::Response>(path_type);
            Some((Run::PluginsInstallFilesystemResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "plugins/install/github" => {
            let parsed = serde_json::from_str::<crate::cli::command::plugins::install::github::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::plugins::install::github::Response>(path_type);
            Some((Run::PluginsInstallGithub { request: parsed, agent_arguments, response }, feed))
        }
        "plugins/install/github/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::plugins::install::github::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::plugins::install::github::request_schema::Response>(path_type);
            Some((Run::PluginsInstallGithubRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "plugins/install/github/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::plugins::install::github::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::plugins::install::github::response_schema::Response>(path_type);
            Some((Run::PluginsInstallGithubResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "plugins/list" => {
            let parsed = serde_json::from_str::<crate::cli::command::plugins::list::Request>(request.get()).ok()?;
            let (response, feed) = stream_feed::<crate::cli::command::plugins::list::ResponseItem>();
            Some((Run::PluginsList { request: parsed, agent_arguments, response }, feed))
        }
        "plugins/list/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::plugins::list::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::plugins::list::request_schema::Response>(path_type);
            Some((Run::PluginsListRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "plugins/list/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::plugins::list::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::plugins::list::response_schema::Response>(path_type);
            Some((Run::PluginsListResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "plugins/logs/list" => {
            let parsed = serde_json::from_str::<crate::cli::command::plugins::logs::list::Request>(request.get()).ok()?;
            let (response, feed) = stream_feed::<crate::cli::command::plugins::logs::list::ResponseItem>();
            Some((Run::PluginsLogsList { request: parsed, agent_arguments, response }, feed))
        }
        "plugins/logs/list/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::plugins::logs::list::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::plugins::logs::list::request_schema::Response>(path_type);
            Some((Run::PluginsLogsListRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "plugins/logs/list/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::plugins::logs::list::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::plugins::logs::list::response_schema::Response>(path_type);
            Some((Run::PluginsLogsListResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "plugins/run" => {
            let parsed = serde_json::from_str::<crate::cli::command::plugins::run::Request>(request.get()).ok()?;
            let (response, feed) = stream_feed::<crate::cli::command::plugins::run::ResponseItem>();
            Some((Run::PluginsRun { request: parsed, agent_arguments, response }, feed))
        }
        "plugins/run/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::plugins::run::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::plugins::run::request_schema::Response>(path_type);
            Some((Run::PluginsRunRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "plugins/run/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::plugins::run::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::plugins::run::response_schema::Response>(path_type);
            Some((Run::PluginsRunResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "python" => {
            let parsed = serde_json::from_str::<crate::cli::command::python::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::python::Response>(path_type);
            Some((Run::Python { request: parsed, agent_arguments, response }, feed))
        }
        "python/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::python::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::python::request_schema::Response>(path_type);
            Some((Run::PythonRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "python/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::python::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::python::response_schema::Response>(path_type);
            Some((Run::PythonResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "swarms/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::swarms::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::swarms::get::Response>(path_type);
            Some((Run::SwarmsGet { request: parsed, agent_arguments, response }, feed))
        }
        "swarms/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::swarms::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::swarms::get::request_schema::Response>(path_type);
            Some((Run::SwarmsGetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "swarms/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::swarms::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::swarms::get::response_schema::Response>(path_type);
            Some((Run::SwarmsGetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "swarms/list" => {
            let parsed = serde_json::from_str::<crate::cli::command::swarms::list::Request>(request.get()).ok()?;
            let (response, feed) = stream_feed::<crate::cli::command::swarms::list::ResponseItem>();
            Some((Run::SwarmsList { request: parsed, agent_arguments, response }, feed))
        }
        "swarms/list/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::swarms::list::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::swarms::list::request_schema::Response>(path_type);
            Some((Run::SwarmsListRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "swarms/list/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::swarms::list::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::swarms::list::response_schema::Response>(path_type);
            Some((Run::SwarmsListResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "swarms/publish" => {
            let parsed = serde_json::from_str::<crate::cli::command::swarms::publish::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::swarms::publish::Response>(path_type);
            Some((Run::SwarmsPublish { request: parsed, agent_arguments, response }, feed))
        }
        "swarms/publish/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::swarms::publish::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::swarms::publish::request_schema::Response>(path_type);
            Some((Run::SwarmsPublishRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "swarms/publish/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::swarms::publish::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::swarms::publish::response_schema::Response>(path_type);
            Some((Run::SwarmsPublishResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "tools/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::tools::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::tools::get::Response>(path_type);
            Some((Run::ToolsGet { request: parsed, agent_arguments, response }, feed))
        }
        "tools/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::tools::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::tools::get::request_schema::Response>(path_type);
            Some((Run::ToolsGetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "tools/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::tools::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::tools::get::response_schema::Response>(path_type);
            Some((Run::ToolsGetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "tools/install/filesystem" => {
            let parsed = serde_json::from_str::<crate::cli::command::tools::install::filesystem::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::tools::install::filesystem::Response>(path_type);
            Some((Run::ToolsInstallFilesystem { request: parsed, agent_arguments, response }, feed))
        }
        "tools/install/filesystem/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::tools::install::filesystem::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::tools::install::filesystem::request_schema::Response>(path_type);
            Some((Run::ToolsInstallFilesystemRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "tools/install/filesystem/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::tools::install::filesystem::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::tools::install::filesystem::response_schema::Response>(path_type);
            Some((Run::ToolsInstallFilesystemResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "tools/install/github" => {
            let parsed = serde_json::from_str::<crate::cli::command::tools::install::github::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::tools::install::github::Response>(path_type);
            Some((Run::ToolsInstallGithub { request: parsed, agent_arguments, response }, feed))
        }
        "tools/install/github/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::tools::install::github::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::tools::install::github::request_schema::Response>(path_type);
            Some((Run::ToolsInstallGithubRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "tools/install/github/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::tools::install::github::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::tools::install::github::response_schema::Response>(path_type);
            Some((Run::ToolsInstallGithubResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "tools/list" => {
            let parsed = serde_json::from_str::<crate::cli::command::tools::list::Request>(request.get()).ok()?;
            let (response, feed) = stream_feed::<crate::cli::command::tools::list::ResponseItem>();
            Some((Run::ToolsList { request: parsed, agent_arguments, response }, feed))
        }
        "tools/list/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::tools::list::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::tools::list::request_schema::Response>(path_type);
            Some((Run::ToolsListRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "tools/list/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::tools::list::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::tools::list::response_schema::Response>(path_type);
            Some((Run::ToolsListResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "tools/run" => {
            let parsed = serde_json::from_str::<crate::cli::command::tools::run::Request>(request.get()).ok()?;
            let (response, feed) = stream_feed::<crate::cli::command::tools::run::ResponseItem>();
            Some((Run::ToolsRun { request: parsed, agent_arguments, response }, feed))
        }
        "tools/run/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::tools::run::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::tools::run::request_schema::Response>(path_type);
            Some((Run::ToolsRunRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "tools/run/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::tools::run::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::tools::run::response_schema::Response>(path_type);
            Some((Run::ToolsRunResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "update" => {
            let parsed = serde_json::from_str::<crate::cli::command::update::Request>(request.get()).ok()?;
            let (response, feed) = stream_feed::<crate::cli::command::update::ResponseItem>();
            Some((Run::Update { request: parsed, agent_arguments, response }, feed))
        }
        "update/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::update::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::update::request_schema::Response>(path_type);
            Some((Run::UpdateRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "update/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::update::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::update::response_schema::Response>(path_type);
            Some((Run::UpdateResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "viewer/config/address/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::viewer::config::address::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::viewer::config::address::get::Response>(path_type);
            Some((Run::ViewerConfigAddressGet { request: parsed, agent_arguments, response }, feed))
        }
        "viewer/config/address/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::viewer::config::address::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::viewer::config::address::get::request_schema::Response>(path_type);
            Some((Run::ViewerConfigAddressGetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "viewer/config/address/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::viewer::config::address::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::viewer::config::address::get::response_schema::Response>(path_type);
            Some((Run::ViewerConfigAddressGetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "viewer/config/address/set" => {
            let parsed = serde_json::from_str::<crate::cli::command::viewer::config::address::set::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::viewer::config::address::set::Response>(path_type);
            Some((Run::ViewerConfigAddressSet { request: parsed, agent_arguments, response }, feed))
        }
        "viewer/config/address/set/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::viewer::config::address::set::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::viewer::config::address::set::request_schema::Response>(path_type);
            Some((Run::ViewerConfigAddressSetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "viewer/config/address/set/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::viewer::config::address::set::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::viewer::config::address::set::response_schema::Response>(path_type);
            Some((Run::ViewerConfigAddressSetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "viewer/config/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::viewer::config::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::viewer::config::get::Response>(path_type);
            Some((Run::ViewerConfigGet { request: parsed, agent_arguments, response }, feed))
        }
        "viewer/config/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::viewer::config::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::viewer::config::get::request_schema::Response>(path_type);
            Some((Run::ViewerConfigGetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "viewer/config/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::viewer::config::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::viewer::config::get::response_schema::Response>(path_type);
            Some((Run::ViewerConfigGetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "viewer/config/secret/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::viewer::config::secret::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::viewer::config::secret::get::Response>(path_type);
            Some((Run::ViewerConfigSecretGet { request: parsed, agent_arguments, response }, feed))
        }
        "viewer/config/secret/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::viewer::config::secret::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::viewer::config::secret::get::request_schema::Response>(path_type);
            Some((Run::ViewerConfigSecretGetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "viewer/config/secret/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::viewer::config::secret::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::viewer::config::secret::get::response_schema::Response>(path_type);
            Some((Run::ViewerConfigSecretGetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "viewer/config/secret/set" => {
            let parsed = serde_json::from_str::<crate::cli::command::viewer::config::secret::set::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::viewer::config::secret::set::Response>(path_type);
            Some((Run::ViewerConfigSecretSet { request: parsed, agent_arguments, response }, feed))
        }
        "viewer/config/secret/set/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::viewer::config::secret::set::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::viewer::config::secret::set::request_schema::Response>(path_type);
            Some((Run::ViewerConfigSecretSetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "viewer/config/secret/set/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::viewer::config::secret::set::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::viewer::config::secret::set::response_schema::Response>(path_type);
            Some((Run::ViewerConfigSecretSetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "viewer/config/signature/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::viewer::config::signature::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::viewer::config::signature::get::Response>(path_type);
            Some((Run::ViewerConfigSignatureGet { request: parsed, agent_arguments, response }, feed))
        }
        "viewer/config/signature/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::viewer::config::signature::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::viewer::config::signature::get::request_schema::Response>(path_type);
            Some((Run::ViewerConfigSignatureGetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "viewer/config/signature/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::viewer::config::signature::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::viewer::config::signature::get::response_schema::Response>(path_type);
            Some((Run::ViewerConfigSignatureGetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "viewer/config/signature/set" => {
            let parsed = serde_json::from_str::<crate::cli::command::viewer::config::signature::set::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::viewer::config::signature::set::Response>(path_type);
            Some((Run::ViewerConfigSignatureSet { request: parsed, agent_arguments, response }, feed))
        }
        "viewer/config/signature/set/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::viewer::config::signature::set::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::viewer::config::signature::set::request_schema::Response>(path_type);
            Some((Run::ViewerConfigSignatureSetRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "viewer/config/signature/set/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::viewer::config::signature::set::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::viewer::config::signature::set::response_schema::Response>(path_type);
            Some((Run::ViewerConfigSignatureSetResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "viewer/generate_secret_signature_pair" => {
            let parsed = serde_json::from_str::<crate::cli::command::viewer::generate_secret_signature_pair::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::viewer::generate_secret_signature_pair::Response>(path_type);
            Some((Run::ViewerGenerateSecretSignaturePair { request: parsed, agent_arguments, response }, feed))
        }
        "viewer/generate_secret_signature_pair/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::viewer::generate_secret_signature_pair::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::viewer::generate_secret_signature_pair::request_schema::Response>(path_type);
            Some((Run::ViewerGenerateSecretSignaturePairRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "viewer/generate_secret_signature_pair/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::viewer::generate_secret_signature_pair::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::viewer::generate_secret_signature_pair::response_schema::Response>(path_type);
            Some((Run::ViewerGenerateSecretSignaturePairResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "viewer/kill" => {
            let parsed = serde_json::from_str::<crate::cli::command::viewer::kill::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::viewer::kill::Response>(path_type);
            Some((Run::ViewerKill { request: parsed, agent_arguments, response }, feed))
        }
        "viewer/kill/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::viewer::kill::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::viewer::kill::request_schema::Response>(path_type);
            Some((Run::ViewerKillRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "viewer/kill/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::viewer::kill::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::viewer::kill::response_schema::Response>(path_type);
            Some((Run::ViewerKillResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        "viewer/spawn" => {
            let parsed = serde_json::from_str::<crate::cli::command::viewer::spawn::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::viewer::spawn::Response>(path_type);
            Some((Run::ViewerSpawn { request: parsed, agent_arguments, response }, feed))
        }
        "viewer/spawn/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::viewer::spawn::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::viewer::spawn::request_schema::Response>(path_type);
            Some((Run::ViewerSpawnRequestSchema { request: parsed, agent_arguments, response }, feed))
        }
        "viewer/spawn/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::viewer::spawn::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::viewer::spawn::response_schema::Response>(path_type);
            Some((Run::ViewerSpawnResponseSchema { request: parsed, agent_arguments, response }, feed))
        }
        _ => None,
    }
}
