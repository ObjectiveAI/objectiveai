//! Wire dispatch for [`super::CommandListener`]: open a run from
//! its broadcast request frame's raw body — discriminate via a probe
//! of the request's `path_type` (plus jq/python and, on the dual-form
//! leaves, `dangerous_advanced.stream`), deserialize the request
//! straight from the wire text as its exact leaf type, and construct
//! the nested [`ListenerExecution`] envelope (the distributed mirror
//! of the `Request` aggregate tree — a struct per leaf, an enum per
//! branch) plus the feed its response frames go through.
//!
//! Bodies arrive as [`RawValue`] wire text and deserialization is
//! deferred until the dispatch knows exactly what type each one is —
//! requests and items parse straight from the wire, never through a
//! `serde_json::Value` intermediate (which would route numbers
//! through `f64` and clobber high-precision decimals).
//!
//! Mechanically authored from the command tree (every scope with a
//! `Path` discriminator; construction chains mirror the `Request`
//! aggregates). A leaf added later and missed here is skipped at
//! runtime; extend the leaf/branch `ListenerExecution` types and this
//! dispatch when touched.

use serde_json::value::RawValue;

use crate::identity::Identity;
use crate::cli::command::ListenerExecution;

use super::listener::{ResponseItemStream, UnaryResponse, decode_item};

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

/// The dispatch probe: the one request field the dispatch needs
/// before it knows the exact type to deserialize the whole request
/// as. Not a wire schema — a borrowed peek at one.
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(serde::Deserialize)]
struct RequestProbe {
    #[serde(default)]
    path_type: Option<String>,
}

/// Open a run from its broadcast request frame's raw body. `None` —
/// an unrecognized `path_type` or a request that fails to
/// deserialize — means the run is skipped entirely.
pub(crate) fn open_run(
    request: &RawValue,
    identity: Identity,
) -> Option<(ListenerExecution, RunFeed)> {
    let probe = serde_json::from_str::<RequestProbe>(request.get()).ok()?;
    let path_type = probe.path_type.as_deref().unwrap_or("");
    match path_type {
        "agents/enqueue" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::enqueue::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::enqueue::Response>(path_type);
            let execution = crate::cli::command::agents::enqueue::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Enqueue(execution)), feed))
        }
        "agents/enqueue/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::enqueue::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::enqueue::request_schema::Response>(path_type);
            let execution = crate::cli::command::agents::enqueue::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::EnqueueRequestSchema(execution)), feed))
        }
        "agents/enqueue/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::enqueue::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::enqueue::response_schema::Response>(path_type);
            let execution = crate::cli::command::agents::enqueue::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::EnqueueResponseSchema(execution)), feed))
        }
        "agents/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::get::Response>(path_type);
            let execution = crate::cli::command::agents::get::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Get(execution)), feed))
        }
        "agents/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::get::request_schema::Response>(path_type);
            let execution = crate::cli::command::agents::get::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::GetRequestSchema(execution)), feed))
        }
        "agents/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::get::response_schema::Response>(path_type);
            let execution = crate::cli::command::agents::get::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::GetResponseSchema(execution)), feed))
        }
        "agents/instances/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::instances::get::Request>(request.get()).ok()?;
            let (response, feed) = stream_feed::<crate::cli::command::agents::instances::get::ResponseItem>();
            let execution = crate::cli::command::agents::instances::get::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Instances(crate::cli::command::agents::instances::ListenerExecution::Get(execution))), feed))
        }
        "agents/instances/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::instances::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::instances::get::request_schema::Response>(path_type);
            let execution = crate::cli::command::agents::instances::get::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Instances(crate::cli::command::agents::instances::ListenerExecution::GetRequestSchema(execution))), feed))
        }
        "agents/instances/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::instances::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::instances::get::response_schema::Response>(path_type);
            let execution = crate::cli::command::agents::instances::get::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Instances(crate::cli::command::agents::instances::ListenerExecution::GetResponseSchema(execution))), feed))
        }
        "agents/instances/list" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::instances::list::Request>(request.get()).ok()?;
            let (response, feed) = stream_feed::<crate::cli::command::agents::instances::list::ResponseItem>();
            let execution = crate::cli::command::agents::instances::list::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Instances(crate::cli::command::agents::instances::ListenerExecution::List(execution))), feed))
        }
        "agents/instances/list/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::instances::list::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::instances::list::request_schema::Response>(path_type);
            let execution = crate::cli::command::agents::instances::list::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Instances(crate::cli::command::agents::instances::ListenerExecution::ListRequestSchema(execution))), feed))
        }
        "agents/instances/list/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::instances::list::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::instances::list::response_schema::Response>(path_type);
            let execution = crate::cli::command::agents::instances::list::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Instances(crate::cli::command::agents::instances::ListenerExecution::ListResponseSchema(execution))), feed))
        }
        "laboratories/attach" => {
            let parsed = serde_json::from_str::<crate::cli::command::laboratories::attach::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::laboratories::attach::Response>(path_type);
            let execution = crate::cli::command::laboratories::attach::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Laboratories(crate::cli::command::laboratories::ListenerExecution::Attach(execution)), feed))
        }
        "laboratories/attach/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::laboratories::attach::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::laboratories::attach::request_schema::Response>(path_type);
            let execution = crate::cli::command::laboratories::attach::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Laboratories(crate::cli::command::laboratories::ListenerExecution::AttachRequestSchema(execution)), feed))
        }
        "laboratories/attach/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::laboratories::attach::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::laboratories::attach::response_schema::Response>(path_type);
            let execution = crate::cli::command::laboratories::attach::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Laboratories(crate::cli::command::laboratories::ListenerExecution::AttachResponseSchema(execution)), feed))
        }
        "laboratories/config/addresses/add" => {
            let parsed = serde_json::from_str::<crate::cli::command::laboratories::config::addresses::add::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::laboratories::config::addresses::add::Response>(path_type);
            let execution = crate::cli::command::laboratories::config::addresses::add::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Laboratories(crate::cli::command::laboratories::ListenerExecution::Config(crate::cli::command::laboratories::config::ListenerExecution::Addresses(crate::cli::command::laboratories::config::addresses::ListenerExecution::Add(execution)))), feed))
        }
        "laboratories/config/addresses/add/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::laboratories::config::addresses::add::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::laboratories::config::addresses::add::request_schema::Response>(path_type);
            let execution = crate::cli::command::laboratories::config::addresses::add::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Laboratories(crate::cli::command::laboratories::ListenerExecution::Config(crate::cli::command::laboratories::config::ListenerExecution::Addresses(crate::cli::command::laboratories::config::addresses::ListenerExecution::AddRequestSchema(execution)))), feed))
        }
        "laboratories/config/addresses/add/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::laboratories::config::addresses::add::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::laboratories::config::addresses::add::response_schema::Response>(path_type);
            let execution = crate::cli::command::laboratories::config::addresses::add::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Laboratories(crate::cli::command::laboratories::ListenerExecution::Config(crate::cli::command::laboratories::config::ListenerExecution::Addresses(crate::cli::command::laboratories::config::addresses::ListenerExecution::AddResponseSchema(execution)))), feed))
        }
        "laboratories/config/addresses/del" => {
            let parsed = serde_json::from_str::<crate::cli::command::laboratories::config::addresses::del::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::laboratories::config::addresses::del::Response>(path_type);
            let execution = crate::cli::command::laboratories::config::addresses::del::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Laboratories(crate::cli::command::laboratories::ListenerExecution::Config(crate::cli::command::laboratories::config::ListenerExecution::Addresses(crate::cli::command::laboratories::config::addresses::ListenerExecution::Del(execution)))), feed))
        }
        "laboratories/config/addresses/del/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::laboratories::config::addresses::del::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::laboratories::config::addresses::del::request_schema::Response>(path_type);
            let execution = crate::cli::command::laboratories::config::addresses::del::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Laboratories(crate::cli::command::laboratories::ListenerExecution::Config(crate::cli::command::laboratories::config::ListenerExecution::Addresses(crate::cli::command::laboratories::config::addresses::ListenerExecution::DelRequestSchema(execution)))), feed))
        }
        "laboratories/config/addresses/del/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::laboratories::config::addresses::del::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::laboratories::config::addresses::del::response_schema::Response>(path_type);
            let execution = crate::cli::command::laboratories::config::addresses::del::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Laboratories(crate::cli::command::laboratories::ListenerExecution::Config(crate::cli::command::laboratories::config::ListenerExecution::Addresses(crate::cli::command::laboratories::config::addresses::ListenerExecution::DelResponseSchema(execution)))), feed))
        }
        "laboratories/config/addresses/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::laboratories::config::addresses::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::laboratories::config::addresses::get::Response>(path_type);
            let execution = crate::cli::command::laboratories::config::addresses::get::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Laboratories(crate::cli::command::laboratories::ListenerExecution::Config(crate::cli::command::laboratories::config::ListenerExecution::Addresses(crate::cli::command::laboratories::config::addresses::ListenerExecution::Get(execution)))), feed))
        }
        "laboratories/config/addresses/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::laboratories::config::addresses::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::laboratories::config::addresses::get::request_schema::Response>(path_type);
            let execution = crate::cli::command::laboratories::config::addresses::get::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Laboratories(crate::cli::command::laboratories::ListenerExecution::Config(crate::cli::command::laboratories::config::ListenerExecution::Addresses(crate::cli::command::laboratories::config::addresses::ListenerExecution::GetRequestSchema(execution)))), feed))
        }
        "laboratories/config/addresses/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::laboratories::config::addresses::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::laboratories::config::addresses::get::response_schema::Response>(path_type);
            let execution = crate::cli::command::laboratories::config::addresses::get::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Laboratories(crate::cli::command::laboratories::ListenerExecution::Config(crate::cli::command::laboratories::config::ListenerExecution::Addresses(crate::cli::command::laboratories::config::addresses::ListenerExecution::GetResponseSchema(execution)))), feed))
        }
        "laboratories/config/local/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::laboratories::config::local::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::laboratories::config::local::get::Response>(path_type);
            let execution = crate::cli::command::laboratories::config::local::get::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Laboratories(crate::cli::command::laboratories::ListenerExecution::Config(crate::cli::command::laboratories::config::ListenerExecution::Local(crate::cli::command::laboratories::config::local::ListenerExecution::Get(execution)))), feed))
        }
        "laboratories/config/local/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::laboratories::config::local::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::laboratories::config::local::get::request_schema::Response>(path_type);
            let execution = crate::cli::command::laboratories::config::local::get::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Laboratories(crate::cli::command::laboratories::ListenerExecution::Config(crate::cli::command::laboratories::config::ListenerExecution::Local(crate::cli::command::laboratories::config::local::ListenerExecution::GetRequestSchema(execution)))), feed))
        }
        "laboratories/config/local/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::laboratories::config::local::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::laboratories::config::local::get::response_schema::Response>(path_type);
            let execution = crate::cli::command::laboratories::config::local::get::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Laboratories(crate::cli::command::laboratories::ListenerExecution::Config(crate::cli::command::laboratories::config::ListenerExecution::Local(crate::cli::command::laboratories::config::local::ListenerExecution::GetResponseSchema(execution)))), feed))
        }
        "laboratories/config/local/set" => {
            let parsed = serde_json::from_str::<crate::cli::command::laboratories::config::local::set::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::laboratories::config::local::set::Response>(path_type);
            let execution = crate::cli::command::laboratories::config::local::set::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Laboratories(crate::cli::command::laboratories::ListenerExecution::Config(crate::cli::command::laboratories::config::ListenerExecution::Local(crate::cli::command::laboratories::config::local::ListenerExecution::Set(execution)))), feed))
        }
        "laboratories/config/local/set/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::laboratories::config::local::set::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::laboratories::config::local::set::request_schema::Response>(path_type);
            let execution = crate::cli::command::laboratories::config::local::set::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Laboratories(crate::cli::command::laboratories::ListenerExecution::Config(crate::cli::command::laboratories::config::ListenerExecution::Local(crate::cli::command::laboratories::config::local::ListenerExecution::SetRequestSchema(execution)))), feed))
        }
        "laboratories/config/local/set/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::laboratories::config::local::set::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::laboratories::config::local::set::response_schema::Response>(path_type);
            let execution = crate::cli::command::laboratories::config::local::set::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Laboratories(crate::cli::command::laboratories::ListenerExecution::Config(crate::cli::command::laboratories::config::ListenerExecution::Local(crate::cli::command::laboratories::config::local::ListenerExecution::SetResponseSchema(execution)))), feed))
        }
        "laboratories/detach" => {
            let parsed = serde_json::from_str::<crate::cli::command::laboratories::detach::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::laboratories::detach::Response>(path_type);
            let execution = crate::cli::command::laboratories::detach::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Laboratories(crate::cli::command::laboratories::ListenerExecution::Detach(execution)), feed))
        }
        "laboratories/detach/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::laboratories::detach::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::laboratories::detach::request_schema::Response>(path_type);
            let execution = crate::cli::command::laboratories::detach::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Laboratories(crate::cli::command::laboratories::ListenerExecution::DetachRequestSchema(execution)), feed))
        }
        "laboratories/detach/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::laboratories::detach::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::laboratories::detach::response_schema::Response>(path_type);
            let execution = crate::cli::command::laboratories::detach::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Laboratories(crate::cli::command::laboratories::ListenerExecution::DetachResponseSchema(execution)), feed))
        }
        "agents/list" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::list::Request>(request.get()).ok()?;
            let (response, feed) = stream_feed::<crate::cli::command::agents::list::ResponseItem>();
            let execution = crate::cli::command::agents::list::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::List(execution)), feed))
        }
        "agents/list/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::list::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::list::request_schema::Response>(path_type);
            let execution = crate::cli::command::agents::list::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::ListRequestSchema(execution)), feed))
        }
        "agents/list/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::list::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::list::response_schema::Response>(path_type);
            let execution = crate::cli::command::agents::list::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::ListResponseSchema(execution)), feed))
        }
        "agents/logs/list" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::logs::list::Request>(request.get()).ok()?;
            let (response, feed) = stream_feed::<crate::cli::command::agents::logs::list::ResponseItem>();
            let execution = crate::cli::command::agents::logs::list::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Logs(crate::cli::command::agents::logs::ListenerExecution::List(execution))), feed))
        }
        "agents/logs/list/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::logs::list::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::logs::list::request_schema::Response>(path_type);
            let execution = crate::cli::command::agents::logs::list::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Logs(crate::cli::command::agents::logs::ListenerExecution::ListRequestSchema(execution))), feed))
        }
        "agents/logs/list/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::logs::list::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::logs::list::response_schema::Response>(path_type);
            let execution = crate::cli::command::agents::logs::list::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Logs(crate::cli::command::agents::logs::ListenerExecution::ListResponseSchema(execution))), feed))
        }
        "agents/logs/open" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::logs::open::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::logs::open::Response>(path_type);
            let execution = crate::cli::command::agents::logs::open::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Logs(crate::cli::command::agents::logs::ListenerExecution::Open(execution))), feed))
        }
        "agents/logs/open/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::logs::open::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::logs::open::request_schema::Response>(path_type);
            let execution = crate::cli::command::agents::logs::open::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Logs(crate::cli::command::agents::logs::ListenerExecution::OpenRequestSchema(execution))), feed))
        }
        "agents/logs/open/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::logs::open::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::logs::open::response_schema::Response>(path_type);
            let execution = crate::cli::command::agents::logs::open::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Logs(crate::cli::command::agents::logs::ListenerExecution::OpenResponseSchema(execution))), feed))
        }
        "agents/logs/subscribe" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::logs::subscribe::Request>(request.get()).ok()?;
            let (response, feed) = stream_feed::<crate::cli::command::agents::logs::subscribe::ResponseItem>();
            let execution = crate::cli::command::agents::logs::subscribe::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Logs(crate::cli::command::agents::logs::ListenerExecution::Subscribe(execution))), feed))
        }
        "agents/logs/subscribe/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::logs::subscribe::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::logs::subscribe::request_schema::Response>(path_type);
            let execution = crate::cli::command::agents::logs::subscribe::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Logs(crate::cli::command::agents::logs::ListenerExecution::SubscribeRequestSchema(execution))), feed))
        }
        "agents/logs/subscribe/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::logs::subscribe::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::logs::subscribe::response_schema::Response>(path_type);
            let execution = crate::cli::command::agents::logs::subscribe::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Logs(crate::cli::command::agents::logs::ListenerExecution::SubscribeResponseSchema(execution))), feed))
        }
        "agents/logs/token-usage/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::logs::token_usage::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::logs::token_usage::get::Response>(path_type);
            let execution = crate::cli::command::agents::logs::token_usage::get::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Logs(crate::cli::command::agents::logs::ListenerExecution::TokenUsage(crate::cli::command::agents::logs::token_usage::ListenerExecution::Get(execution)))), feed))
        }
        "agents/logs/token-usage/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::logs::token_usage::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::logs::token_usage::get::request_schema::Response>(path_type);
            let execution = crate::cli::command::agents::logs::token_usage::get::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Logs(crate::cli::command::agents::logs::ListenerExecution::TokenUsage(crate::cli::command::agents::logs::token_usage::ListenerExecution::GetRequestSchema(execution)))), feed))
        }
        "agents/logs/token-usage/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::logs::token_usage::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::logs::token_usage::get::response_schema::Response>(path_type);
            let execution = crate::cli::command::agents::logs::token_usage::get::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Logs(crate::cli::command::agents::logs::ListenerExecution::TokenUsage(crate::cli::command::agents::logs::token_usage::ListenerExecution::GetResponseSchema(execution)))), feed))
        }
        "agents/logs/token-usage/subscribe" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::logs::token_usage::subscribe::Request>(request.get()).ok()?;
            let (response, feed) = stream_feed::<crate::cli::command::agents::logs::token_usage::subscribe::ResponseItem>();
            let execution = crate::cli::command::agents::logs::token_usage::subscribe::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Logs(crate::cli::command::agents::logs::ListenerExecution::TokenUsage(crate::cli::command::agents::logs::token_usage::ListenerExecution::Subscribe(execution)))), feed))
        }
        "agents/logs/token-usage/subscribe/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::logs::token_usage::subscribe::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::logs::token_usage::subscribe::request_schema::Response>(path_type);
            let execution = crate::cli::command::agents::logs::token_usage::subscribe::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Logs(crate::cli::command::agents::logs::ListenerExecution::TokenUsage(crate::cli::command::agents::logs::token_usage::ListenerExecution::SubscribeRequestSchema(execution)))), feed))
        }
        "agents/logs/token-usage/subscribe/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::logs::token_usage::subscribe::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::logs::token_usage::subscribe::response_schema::Response>(path_type);
            let execution = crate::cli::command::agents::logs::token_usage::subscribe::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Logs(crate::cli::command::agents::logs::ListenerExecution::TokenUsage(crate::cli::command::agents::logs::token_usage::ListenerExecution::SubscribeResponseSchema(execution)))), feed))
        }
        "agents/mcp/resources/list" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::mcp::resources::list::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::mcp::resources::list::Response>(path_type);
            let execution = crate::cli::command::agents::mcp::resources::list::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Mcp(crate::cli::command::agents::mcp::ListenerExecution::Resources(crate::cli::command::agents::mcp::resources::ListenerExecution::List(execution)))), feed))
        }
        "agents/mcp/resources/list/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::mcp::resources::list::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::mcp::resources::list::request_schema::Response>(path_type);
            let execution = crate::cli::command::agents::mcp::resources::list::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Mcp(crate::cli::command::agents::mcp::ListenerExecution::Resources(crate::cli::command::agents::mcp::resources::ListenerExecution::ListRequestSchema(execution)))), feed))
        }
        "agents/mcp/resources/list/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::mcp::resources::list::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::mcp::resources::list::response_schema::Response>(path_type);
            let execution = crate::cli::command::agents::mcp::resources::list::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Mcp(crate::cli::command::agents::mcp::ListenerExecution::Resources(crate::cli::command::agents::mcp::resources::ListenerExecution::ListResponseSchema(execution)))), feed))
        }
        "agents/mcp/resources/read" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::mcp::resources::read::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::mcp::resources::read::Response>(path_type);
            let execution = crate::cli::command::agents::mcp::resources::read::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Mcp(crate::cli::command::agents::mcp::ListenerExecution::Resources(crate::cli::command::agents::mcp::resources::ListenerExecution::Read(execution)))), feed))
        }
        "agents/mcp/resources/read/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::mcp::resources::read::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::mcp::resources::read::request_schema::Response>(path_type);
            let execution = crate::cli::command::agents::mcp::resources::read::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Mcp(crate::cli::command::agents::mcp::ListenerExecution::Resources(crate::cli::command::agents::mcp::resources::ListenerExecution::ReadRequestSchema(execution)))), feed))
        }
        "agents/mcp/resources/read/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::mcp::resources::read::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::mcp::resources::read::response_schema::Response>(path_type);
            let execution = crate::cli::command::agents::mcp::resources::read::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Mcp(crate::cli::command::agents::mcp::ListenerExecution::Resources(crate::cli::command::agents::mcp::resources::ListenerExecution::ReadResponseSchema(execution)))), feed))
        }
        "agents/mcp/servers/list" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::mcp::servers::list::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::mcp::servers::list::Response>(path_type);
            let execution = crate::cli::command::agents::mcp::servers::list::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Mcp(crate::cli::command::agents::mcp::ListenerExecution::Servers(crate::cli::command::agents::mcp::servers::ListenerExecution::List(execution)))), feed))
        }
        "agents/mcp/servers/list/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::mcp::servers::list::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::mcp::servers::list::request_schema::Response>(path_type);
            let execution = crate::cli::command::agents::mcp::servers::list::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Mcp(crate::cli::command::agents::mcp::ListenerExecution::Servers(crate::cli::command::agents::mcp::servers::ListenerExecution::ListRequestSchema(execution)))), feed))
        }
        "agents/mcp/servers/list/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::mcp::servers::list::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::mcp::servers::list::response_schema::Response>(path_type);
            let execution = crate::cli::command::agents::mcp::servers::list::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Mcp(crate::cli::command::agents::mcp::ListenerExecution::Servers(crate::cli::command::agents::mcp::servers::ListenerExecution::ListResponseSchema(execution)))), feed))
        }
        "agents/mcp/tools/call" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::mcp::tools::call::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::mcp::tools::call::Response>(path_type);
            let execution = crate::cli::command::agents::mcp::tools::call::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Mcp(crate::cli::command::agents::mcp::ListenerExecution::Tools(crate::cli::command::agents::mcp::tools::ListenerExecution::Call(execution)))), feed))
        }
        "agents/mcp/tools/call/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::mcp::tools::call::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::mcp::tools::call::request_schema::Response>(path_type);
            let execution = crate::cli::command::agents::mcp::tools::call::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Mcp(crate::cli::command::agents::mcp::ListenerExecution::Tools(crate::cli::command::agents::mcp::tools::ListenerExecution::CallRequestSchema(execution)))), feed))
        }
        "agents/mcp/tools/call/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::mcp::tools::call::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::mcp::tools::call::response_schema::Response>(path_type);
            let execution = crate::cli::command::agents::mcp::tools::call::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Mcp(crate::cli::command::agents::mcp::ListenerExecution::Tools(crate::cli::command::agents::mcp::tools::ListenerExecution::CallResponseSchema(execution)))), feed))
        }
        "agents/mcp/tools/list" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::mcp::tools::list::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::mcp::tools::list::Response>(path_type);
            let execution = crate::cli::command::agents::mcp::tools::list::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Mcp(crate::cli::command::agents::mcp::ListenerExecution::Tools(crate::cli::command::agents::mcp::tools::ListenerExecution::List(execution)))), feed))
        }
        "agents/mcp/tools/list/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::mcp::tools::list::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::mcp::tools::list::request_schema::Response>(path_type);
            let execution = crate::cli::command::agents::mcp::tools::list::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Mcp(crate::cli::command::agents::mcp::ListenerExecution::Tools(crate::cli::command::agents::mcp::tools::ListenerExecution::ListRequestSchema(execution)))), feed))
        }
        "agents/mcp/tools/list/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::mcp::tools::list::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::mcp::tools::list::response_schema::Response>(path_type);
            let execution = crate::cli::command::agents::mcp::tools::list::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Mcp(crate::cli::command::agents::mcp::ListenerExecution::Tools(crate::cli::command::agents::mcp::tools::ListenerExecution::ListResponseSchema(execution)))), feed))
        }
        "agents/message" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::message::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::message::Response>(path_type);
            let execution = crate::cli::command::agents::message::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Message(execution)), feed))
        }
        "agents/message/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::message::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::message::request_schema::Response>(path_type);
            let execution = crate::cli::command::agents::message::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::MessageRequestSchema(execution)), feed))
        }
        "agents/message/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::message::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::message::response_schema::Response>(path_type);
            let execution = crate::cli::command::agents::message::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::MessageResponseSchema(execution)), feed))
        }
        "agents/publish" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::publish::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::publish::Response>(path_type);
            let execution = crate::cli::command::agents::publish::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Publish(execution)), feed))
        }
        "agents/publish/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::publish::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::publish::request_schema::Response>(path_type);
            let execution = crate::cli::command::agents::publish::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::PublishRequestSchema(execution)), feed))
        }
        "agents/publish/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::publish::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::publish::response_schema::Response>(path_type);
            let execution = crate::cli::command::agents::publish::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::PublishResponseSchema(execution)), feed))
        }
        "agents/queue/delete" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::queue::delete::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::queue::delete::Response>(path_type);
            let execution = crate::cli::command::agents::queue::delete::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Queue(crate::cli::command::agents::queue::ListenerExecution::Delete(execution))), feed))
        }
        "agents/queue/delete/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::queue::delete::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::queue::delete::request_schema::Response>(path_type);
            let execution = crate::cli::command::agents::queue::delete::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Queue(crate::cli::command::agents::queue::ListenerExecution::DeleteRequestSchema(execution))), feed))
        }
        "agents/queue/delete/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::queue::delete::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::queue::delete::response_schema::Response>(path_type);
            let execution = crate::cli::command::agents::queue::delete::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Queue(crate::cli::command::agents::queue::ListenerExecution::DeleteResponseSchema(execution))), feed))
        }
        "agents/queue/deliver" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::queue::deliver::Request>(request.get()).ok()?;
            let (response, feed) = stream_feed::<crate::cli::command::agents::queue::deliver::ResponseItem>();
            let execution = crate::cli::command::agents::queue::deliver::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Queue(crate::cli::command::agents::queue::ListenerExecution::Deliver(execution))), feed))
        }
        "agents/queue/deliver/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::queue::deliver::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::queue::deliver::request_schema::Response>(path_type);
            let execution = crate::cli::command::agents::queue::deliver::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Queue(crate::cli::command::agents::queue::ListenerExecution::DeliverRequestSchema(execution))), feed))
        }
        "agents/queue/deliver/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::queue::deliver::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::queue::deliver::response_schema::Response>(path_type);
            let execution = crate::cli::command::agents::queue::deliver::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Queue(crate::cli::command::agents::queue::ListenerExecution::DeliverResponseSchema(execution))), feed))
        }
        "agents/queue/list" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::queue::list::Request>(request.get()).ok()?;
            let (response, feed) = stream_feed::<crate::cli::command::agents::queue::list::ResponseItem>();
            let execution = crate::cli::command::agents::queue::list::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Queue(crate::cli::command::agents::queue::ListenerExecution::List(execution))), feed))
        }
        "agents/queue/list/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::queue::list::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::queue::list::request_schema::Response>(path_type);
            let execution = crate::cli::command::agents::queue::list::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Queue(crate::cli::command::agents::queue::ListenerExecution::ListRequestSchema(execution))), feed))
        }
        "agents/queue/list/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::queue::list::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::queue::list::response_schema::Response>(path_type);
            let execution = crate::cli::command::agents::queue::list::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Queue(crate::cli::command::agents::queue::ListenerExecution::ListResponseSchema(execution))), feed))
        }
        "agents/queue/open" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::queue::open::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::queue::open::Response>(path_type);
            let execution = crate::cli::command::agents::queue::open::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Queue(crate::cli::command::agents::queue::ListenerExecution::Open(execution))), feed))
        }
        "agents/queue/open/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::queue::open::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::queue::open::request_schema::Response>(path_type);
            let execution = crate::cli::command::agents::queue::open::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Queue(crate::cli::command::agents::queue::ListenerExecution::OpenRequestSchema(execution))), feed))
        }
        "agents/queue/open/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::queue::open::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::queue::open::response_schema::Response>(path_type);
            let execution = crate::cli::command::agents::queue::open::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Queue(crate::cli::command::agents::queue::ListenerExecution::OpenResponseSchema(execution))), feed))
        }
        "agents/spawn" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::spawn::Request>(request.get()).ok()?;
            let (variant, feed) = if parsed.dangerous_advanced.as_ref().and_then(|a| a.stream) == Some(true) {
                let (response, feed) = stream_feed::<crate::cli::command::agents::spawn::ResponseItem>();
                let execution = crate::cli::command::agents::spawn::ListenerExecutionStreaming { request: parsed, identity, response };
                (crate::cli::command::agents::spawn::ListenerExecutionVariant::Streaming(execution), feed)
            } else {
                let (response, feed) = unary_feed::<crate::cli::command::agents::spawn::Response>(path_type);
                let execution = crate::cli::command::agents::spawn::ListenerExecution { request: parsed, identity, response };
                (crate::cli::command::agents::spawn::ListenerExecutionVariant::Execution(execution), feed)
            };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Spawn(variant)), feed))
        }
        "agents/spawn/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::spawn::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::spawn::request_schema::Response>(path_type);
            let execution = crate::cli::command::agents::spawn::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::SpawnRequestSchema(execution)), feed))
        }
        "agents/spawn/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::spawn::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::spawn::response_schema::Response>(path_type);
            let execution = crate::cli::command::agents::spawn::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::SpawnResponseSchema(execution)), feed))
        }
        "agents/tags/apply" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::tags::apply::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::tags::apply::Response>(path_type);
            let execution = crate::cli::command::agents::tags::apply::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Tags(crate::cli::command::agents::tags::ListenerExecution::Apply(execution))), feed))
        }
        "agents/tags/apply/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::tags::apply::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::tags::apply::request_schema::Response>(path_type);
            let execution = crate::cli::command::agents::tags::apply::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Tags(crate::cli::command::agents::tags::ListenerExecution::ApplyRequestSchema(execution))), feed))
        }
        "agents/tags/apply/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::tags::apply::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::tags::apply::response_schema::Response>(path_type);
            let execution = crate::cli::command::agents::tags::apply::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Tags(crate::cli::command::agents::tags::ListenerExecution::ApplyResponseSchema(execution))), feed))
        }
        "agents/tags/lookup" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::tags::lookup::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::tags::lookup::Response>(path_type);
            let execution = crate::cli::command::agents::tags::lookup::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Tags(crate::cli::command::agents::tags::ListenerExecution::Lookup(execution))), feed))
        }
        "agents/tags/lookup/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::tags::lookup::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::tags::lookup::request_schema::Response>(path_type);
            let execution = crate::cli::command::agents::tags::lookup::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Tags(crate::cli::command::agents::tags::ListenerExecution::LookupRequestSchema(execution))), feed))
        }
        "agents/tags/lookup/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::tags::lookup::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::tags::lookup::response_schema::Response>(path_type);
            let execution = crate::cli::command::agents::tags::lookup::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Tags(crate::cli::command::agents::tags::ListenerExecution::LookupResponseSchema(execution))), feed))
        }
        "agents/tags/remove" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::tags::remove::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::tags::remove::Response>(path_type);
            let execution = crate::cli::command::agents::tags::remove::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Tags(crate::cli::command::agents::tags::ListenerExecution::Remove(execution))), feed))
        }
        "agents/tags/remove/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::tags::remove::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::tags::remove::request_schema::Response>(path_type);
            let execution = crate::cli::command::agents::tags::remove::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Tags(crate::cli::command::agents::tags::ListenerExecution::RemoveRequestSchema(execution))), feed))
        }
        "agents/tags/remove/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::tags::remove::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::tags::remove::response_schema::Response>(path_type);
            let execution = crate::cli::command::agents::tags::remove::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Tags(crate::cli::command::agents::tags::ListenerExecution::RemoveResponseSchema(execution))), feed))
        }
        "agents/wait" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::wait::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::wait::Response>(path_type);
            let execution = crate::cli::command::agents::wait::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::Wait(execution)), feed))
        }
        "agents/wait/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::wait::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::wait::request_schema::Response>(path_type);
            let execution = crate::cli::command::agents::wait::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::WaitRequestSchema(execution)), feed))
        }
        "agents/wait/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::agents::wait::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::agents::wait::response_schema::Response>(path_type);
            let execution = crate::cli::command::agents::wait::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Agents(crate::cli::command::agents::ListenerExecution::WaitResponseSchema(execution)), feed))
        }
        "api/config/address/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::address::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::address::get::Response>(path_type);
            let execution = crate::cli::command::api::config::address::get::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::Address(crate::cli::command::api::config::address::ListenerExecution::Get(execution)))), feed))
        }
        "api/config/address/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::address::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::address::get::request_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::address::get::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::Address(crate::cli::command::api::config::address::ListenerExecution::GetRequestSchema(execution)))), feed))
        }
        "api/config/address/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::address::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::address::get::response_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::address::get::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::Address(crate::cli::command::api::config::address::ListenerExecution::GetResponseSchema(execution)))), feed))
        }
        "api/config/address/set" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::address::set::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::address::set::Response>(path_type);
            let execution = crate::cli::command::api::config::address::set::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::Address(crate::cli::command::api::config::address::ListenerExecution::Set(execution)))), feed))
        }
        "api/config/address/set/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::address::set::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::address::set::request_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::address::set::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::Address(crate::cli::command::api::config::address::ListenerExecution::SetRequestSchema(execution)))), feed))
        }
        "api/config/address/set/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::address::set::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::address::set::response_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::address::set::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::Address(crate::cli::command::api::config::address::ListenerExecution::SetResponseSchema(execution)))), feed))
        }
        "api/config/backoff_max_elapsed_time_ms/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::backoff_max_elapsed_time_ms::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::backoff_max_elapsed_time_ms::get::Response>(path_type);
            let execution = crate::cli::command::api::config::backoff_max_elapsed_time_ms::get::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::BackoffMaxElapsedTimeMs(crate::cli::command::api::config::backoff_max_elapsed_time_ms::ListenerExecution::Get(execution)))), feed))
        }
        "api/config/backoff_max_elapsed_time_ms/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::backoff_max_elapsed_time_ms::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::backoff_max_elapsed_time_ms::get::request_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::backoff_max_elapsed_time_ms::get::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::BackoffMaxElapsedTimeMs(crate::cli::command::api::config::backoff_max_elapsed_time_ms::ListenerExecution::GetRequestSchema(execution)))), feed))
        }
        "api/config/backoff_max_elapsed_time_ms/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::backoff_max_elapsed_time_ms::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::backoff_max_elapsed_time_ms::get::response_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::backoff_max_elapsed_time_ms::get::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::BackoffMaxElapsedTimeMs(crate::cli::command::api::config::backoff_max_elapsed_time_ms::ListenerExecution::GetResponseSchema(execution)))), feed))
        }
        "api/config/backoff_max_elapsed_time_ms/set" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::backoff_max_elapsed_time_ms::set::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::backoff_max_elapsed_time_ms::set::Response>(path_type);
            let execution = crate::cli::command::api::config::backoff_max_elapsed_time_ms::set::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::BackoffMaxElapsedTimeMs(crate::cli::command::api::config::backoff_max_elapsed_time_ms::ListenerExecution::Set(execution)))), feed))
        }
        "api/config/backoff_max_elapsed_time_ms/set/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::backoff_max_elapsed_time_ms::set::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::backoff_max_elapsed_time_ms::set::request_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::backoff_max_elapsed_time_ms::set::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::BackoffMaxElapsedTimeMs(crate::cli::command::api::config::backoff_max_elapsed_time_ms::ListenerExecution::SetRequestSchema(execution)))), feed))
        }
        "api/config/backoff_max_elapsed_time_ms/set/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::backoff_max_elapsed_time_ms::set::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::backoff_max_elapsed_time_ms::set::response_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::backoff_max_elapsed_time_ms::set::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::BackoffMaxElapsedTimeMs(crate::cli::command::api::config::backoff_max_elapsed_time_ms::ListenerExecution::SetResponseSchema(execution)))), feed))
        }
        "api/config/commit_author_email/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::commit_author_email::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::commit_author_email::get::Response>(path_type);
            let execution = crate::cli::command::api::config::commit_author_email::get::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::CommitAuthorEmail(crate::cli::command::api::config::commit_author_email::ListenerExecution::Get(execution)))), feed))
        }
        "api/config/commit_author_email/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::commit_author_email::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::commit_author_email::get::request_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::commit_author_email::get::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::CommitAuthorEmail(crate::cli::command::api::config::commit_author_email::ListenerExecution::GetRequestSchema(execution)))), feed))
        }
        "api/config/commit_author_email/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::commit_author_email::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::commit_author_email::get::response_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::commit_author_email::get::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::CommitAuthorEmail(crate::cli::command::api::config::commit_author_email::ListenerExecution::GetResponseSchema(execution)))), feed))
        }
        "api/config/commit_author_email/set" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::commit_author_email::set::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::commit_author_email::set::Response>(path_type);
            let execution = crate::cli::command::api::config::commit_author_email::set::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::CommitAuthorEmail(crate::cli::command::api::config::commit_author_email::ListenerExecution::Set(execution)))), feed))
        }
        "api/config/commit_author_email/set/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::commit_author_email::set::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::commit_author_email::set::request_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::commit_author_email::set::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::CommitAuthorEmail(crate::cli::command::api::config::commit_author_email::ListenerExecution::SetRequestSchema(execution)))), feed))
        }
        "api/config/commit_author_email/set/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::commit_author_email::set::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::commit_author_email::set::response_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::commit_author_email::set::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::CommitAuthorEmail(crate::cli::command::api::config::commit_author_email::ListenerExecution::SetResponseSchema(execution)))), feed))
        }
        "api/config/commit_author_name/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::commit_author_name::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::commit_author_name::get::Response>(path_type);
            let execution = crate::cli::command::api::config::commit_author_name::get::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::CommitAuthorName(crate::cli::command::api::config::commit_author_name::ListenerExecution::Get(execution)))), feed))
        }
        "api/config/commit_author_name/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::commit_author_name::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::commit_author_name::get::request_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::commit_author_name::get::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::CommitAuthorName(crate::cli::command::api::config::commit_author_name::ListenerExecution::GetRequestSchema(execution)))), feed))
        }
        "api/config/commit_author_name/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::commit_author_name::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::commit_author_name::get::response_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::commit_author_name::get::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::CommitAuthorName(crate::cli::command::api::config::commit_author_name::ListenerExecution::GetResponseSchema(execution)))), feed))
        }
        "api/config/commit_author_name/set" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::commit_author_name::set::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::commit_author_name::set::Response>(path_type);
            let execution = crate::cli::command::api::config::commit_author_name::set::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::CommitAuthorName(crate::cli::command::api::config::commit_author_name::ListenerExecution::Set(execution)))), feed))
        }
        "api/config/commit_author_name/set/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::commit_author_name::set::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::commit_author_name::set::request_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::commit_author_name::set::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::CommitAuthorName(crate::cli::command::api::config::commit_author_name::ListenerExecution::SetRequestSchema(execution)))), feed))
        }
        "api/config/commit_author_name/set/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::commit_author_name::set::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::commit_author_name::set::response_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::commit_author_name::set::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::CommitAuthorName(crate::cli::command::api::config::commit_author_name::ListenerExecution::SetResponseSchema(execution)))), feed))
        }
        "api/config/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::get::Response>(path_type);
            let execution = crate::cli::command::api::config::get::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::Get(execution))), feed))
        }
        "api/config/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::get::request_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::get::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::GetRequestSchema(execution))), feed))
        }
        "api/config/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::get::response_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::get::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::GetResponseSchema(execution))), feed))
        }
        "api/config/github_authorization/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::github_authorization::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::github_authorization::get::Response>(path_type);
            let execution = crate::cli::command::api::config::github_authorization::get::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::GithubAuthorization(crate::cli::command::api::config::github_authorization::ListenerExecution::Get(execution)))), feed))
        }
        "api/config/github_authorization/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::github_authorization::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::github_authorization::get::request_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::github_authorization::get::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::GithubAuthorization(crate::cli::command::api::config::github_authorization::ListenerExecution::GetRequestSchema(execution)))), feed))
        }
        "api/config/github_authorization/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::github_authorization::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::github_authorization::get::response_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::github_authorization::get::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::GithubAuthorization(crate::cli::command::api::config::github_authorization::ListenerExecution::GetResponseSchema(execution)))), feed))
        }
        "api/config/github_authorization/set" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::github_authorization::set::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::github_authorization::set::Response>(path_type);
            let execution = crate::cli::command::api::config::github_authorization::set::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::GithubAuthorization(crate::cli::command::api::config::github_authorization::ListenerExecution::Set(execution)))), feed))
        }
        "api/config/github_authorization/set/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::github_authorization::set::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::github_authorization::set::request_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::github_authorization::set::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::GithubAuthorization(crate::cli::command::api::config::github_authorization::ListenerExecution::SetRequestSchema(execution)))), feed))
        }
        "api/config/github_authorization/set/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::github_authorization::set::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::github_authorization::set::response_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::github_authorization::set::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::GithubAuthorization(crate::cli::command::api::config::github_authorization::ListenerExecution::SetResponseSchema(execution)))), feed))
        }
        "api/config/http_referer/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::http_referer::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::http_referer::get::Response>(path_type);
            let execution = crate::cli::command::api::config::http_referer::get::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::HttpReferer(crate::cli::command::api::config::http_referer::ListenerExecution::Get(execution)))), feed))
        }
        "api/config/http_referer/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::http_referer::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::http_referer::get::request_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::http_referer::get::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::HttpReferer(crate::cli::command::api::config::http_referer::ListenerExecution::GetRequestSchema(execution)))), feed))
        }
        "api/config/http_referer/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::http_referer::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::http_referer::get::response_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::http_referer::get::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::HttpReferer(crate::cli::command::api::config::http_referer::ListenerExecution::GetResponseSchema(execution)))), feed))
        }
        "api/config/http_referer/set" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::http_referer::set::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::http_referer::set::Response>(path_type);
            let execution = crate::cli::command::api::config::http_referer::set::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::HttpReferer(crate::cli::command::api::config::http_referer::ListenerExecution::Set(execution)))), feed))
        }
        "api/config/http_referer/set/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::http_referer::set::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::http_referer::set::request_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::http_referer::set::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::HttpReferer(crate::cli::command::api::config::http_referer::ListenerExecution::SetRequestSchema(execution)))), feed))
        }
        "api/config/http_referer/set/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::http_referer::set::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::http_referer::set::response_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::http_referer::set::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::HttpReferer(crate::cli::command::api::config::http_referer::ListenerExecution::SetResponseSchema(execution)))), feed))
        }
        "api/config/mcp_authorization/add" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::mcp_authorization::add::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::mcp_authorization::add::Response>(path_type);
            let execution = crate::cli::command::api::config::mcp_authorization::add::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::McpAuthorization(crate::cli::command::api::config::mcp_authorization::ListenerExecution::Add(execution)))), feed))
        }
        "api/config/mcp_authorization/add/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::mcp_authorization::add::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::mcp_authorization::add::request_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::mcp_authorization::add::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::McpAuthorization(crate::cli::command::api::config::mcp_authorization::ListenerExecution::AddRequestSchema(execution)))), feed))
        }
        "api/config/mcp_authorization/add/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::mcp_authorization::add::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::mcp_authorization::add::response_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::mcp_authorization::add::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::McpAuthorization(crate::cli::command::api::config::mcp_authorization::ListenerExecution::AddResponseSchema(execution)))), feed))
        }
        "api/config/mcp_authorization/del" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::mcp_authorization::del::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::mcp_authorization::del::Response>(path_type);
            let execution = crate::cli::command::api::config::mcp_authorization::del::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::McpAuthorization(crate::cli::command::api::config::mcp_authorization::ListenerExecution::Del(execution)))), feed))
        }
        "api/config/mcp_authorization/del/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::mcp_authorization::del::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::mcp_authorization::del::request_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::mcp_authorization::del::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::McpAuthorization(crate::cli::command::api::config::mcp_authorization::ListenerExecution::DelRequestSchema(execution)))), feed))
        }
        "api/config/mcp_authorization/del/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::mcp_authorization::del::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::mcp_authorization::del::response_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::mcp_authorization::del::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::McpAuthorization(crate::cli::command::api::config::mcp_authorization::ListenerExecution::DelResponseSchema(execution)))), feed))
        }
        "api/config/mcp_authorization/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::mcp_authorization::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::mcp_authorization::get::Response>(path_type);
            let execution = crate::cli::command::api::config::mcp_authorization::get::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::McpAuthorization(crate::cli::command::api::config::mcp_authorization::ListenerExecution::Get(execution)))), feed))
        }
        "api/config/mcp_authorization/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::mcp_authorization::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::mcp_authorization::get::request_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::mcp_authorization::get::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::McpAuthorization(crate::cli::command::api::config::mcp_authorization::ListenerExecution::GetRequestSchema(execution)))), feed))
        }
        "api/config/mcp_authorization/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::mcp_authorization::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::mcp_authorization::get::response_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::mcp_authorization::get::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::McpAuthorization(crate::cli::command::api::config::mcp_authorization::ListenerExecution::GetResponseSchema(execution)))), feed))
        }
        "api/config/mcp_call_timeout_ms/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::mcp_call_timeout_ms::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::mcp_call_timeout_ms::get::Response>(path_type);
            let execution = crate::cli::command::api::config::mcp_call_timeout_ms::get::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::McpCallTimeoutMs(crate::cli::command::api::config::mcp_call_timeout_ms::ListenerExecution::Get(execution)))), feed))
        }
        "api/config/mcp_call_timeout_ms/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::mcp_call_timeout_ms::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::mcp_call_timeout_ms::get::request_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::mcp_call_timeout_ms::get::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::McpCallTimeoutMs(crate::cli::command::api::config::mcp_call_timeout_ms::ListenerExecution::GetRequestSchema(execution)))), feed))
        }
        "api/config/mcp_call_timeout_ms/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::mcp_call_timeout_ms::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::mcp_call_timeout_ms::get::response_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::mcp_call_timeout_ms::get::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::McpCallTimeoutMs(crate::cli::command::api::config::mcp_call_timeout_ms::ListenerExecution::GetResponseSchema(execution)))), feed))
        }
        "api/config/mcp_call_timeout_ms/set" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::mcp_call_timeout_ms::set::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::mcp_call_timeout_ms::set::Response>(path_type);
            let execution = crate::cli::command::api::config::mcp_call_timeout_ms::set::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::McpCallTimeoutMs(crate::cli::command::api::config::mcp_call_timeout_ms::ListenerExecution::Set(execution)))), feed))
        }
        "api/config/mcp_call_timeout_ms/set/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::mcp_call_timeout_ms::set::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::mcp_call_timeout_ms::set::request_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::mcp_call_timeout_ms::set::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::McpCallTimeoutMs(crate::cli::command::api::config::mcp_call_timeout_ms::ListenerExecution::SetRequestSchema(execution)))), feed))
        }
        "api/config/mcp_call_timeout_ms/set/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::mcp_call_timeout_ms::set::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::mcp_call_timeout_ms::set::response_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::mcp_call_timeout_ms::set::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::McpCallTimeoutMs(crate::cli::command::api::config::mcp_call_timeout_ms::ListenerExecution::SetResponseSchema(execution)))), feed))
        }
        "api/config/mcp_connect_timeout_ms/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::mcp_connect_timeout_ms::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::mcp_connect_timeout_ms::get::Response>(path_type);
            let execution = crate::cli::command::api::config::mcp_connect_timeout_ms::get::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::McpConnectTimeoutMs(crate::cli::command::api::config::mcp_connect_timeout_ms::ListenerExecution::Get(execution)))), feed))
        }
        "api/config/mcp_connect_timeout_ms/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::mcp_connect_timeout_ms::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::mcp_connect_timeout_ms::get::request_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::mcp_connect_timeout_ms::get::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::McpConnectTimeoutMs(crate::cli::command::api::config::mcp_connect_timeout_ms::ListenerExecution::GetRequestSchema(execution)))), feed))
        }
        "api/config/mcp_connect_timeout_ms/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::mcp_connect_timeout_ms::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::mcp_connect_timeout_ms::get::response_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::mcp_connect_timeout_ms::get::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::McpConnectTimeoutMs(crate::cli::command::api::config::mcp_connect_timeout_ms::ListenerExecution::GetResponseSchema(execution)))), feed))
        }
        "api/config/mcp_connect_timeout_ms/set" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::mcp_connect_timeout_ms::set::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::mcp_connect_timeout_ms::set::Response>(path_type);
            let execution = crate::cli::command::api::config::mcp_connect_timeout_ms::set::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::McpConnectTimeoutMs(crate::cli::command::api::config::mcp_connect_timeout_ms::ListenerExecution::Set(execution)))), feed))
        }
        "api/config/mcp_connect_timeout_ms/set/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::mcp_connect_timeout_ms::set::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::mcp_connect_timeout_ms::set::request_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::mcp_connect_timeout_ms::set::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::McpConnectTimeoutMs(crate::cli::command::api::config::mcp_connect_timeout_ms::ListenerExecution::SetRequestSchema(execution)))), feed))
        }
        "api/config/mcp_connect_timeout_ms/set/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::mcp_connect_timeout_ms::set::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::mcp_connect_timeout_ms::set::response_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::mcp_connect_timeout_ms::set::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::McpConnectTimeoutMs(crate::cli::command::api::config::mcp_connect_timeout_ms::ListenerExecution::SetResponseSchema(execution)))), feed))
        }
        "api/config/objectiveai_authorization/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::objectiveai_authorization::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::objectiveai_authorization::get::Response>(path_type);
            let execution = crate::cli::command::api::config::objectiveai_authorization::get::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::ObjectiveaiAuthorization(crate::cli::command::api::config::objectiveai_authorization::ListenerExecution::Get(execution)))), feed))
        }
        "api/config/objectiveai_authorization/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::objectiveai_authorization::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::objectiveai_authorization::get::request_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::objectiveai_authorization::get::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::ObjectiveaiAuthorization(crate::cli::command::api::config::objectiveai_authorization::ListenerExecution::GetRequestSchema(execution)))), feed))
        }
        "api/config/objectiveai_authorization/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::objectiveai_authorization::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::objectiveai_authorization::get::response_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::objectiveai_authorization::get::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::ObjectiveaiAuthorization(crate::cli::command::api::config::objectiveai_authorization::ListenerExecution::GetResponseSchema(execution)))), feed))
        }
        "api/config/objectiveai_authorization/set" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::objectiveai_authorization::set::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::objectiveai_authorization::set::Response>(path_type);
            let execution = crate::cli::command::api::config::objectiveai_authorization::set::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::ObjectiveaiAuthorization(crate::cli::command::api::config::objectiveai_authorization::ListenerExecution::Set(execution)))), feed))
        }
        "api/config/objectiveai_authorization/set/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::objectiveai_authorization::set::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::objectiveai_authorization::set::request_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::objectiveai_authorization::set::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::ObjectiveaiAuthorization(crate::cli::command::api::config::objectiveai_authorization::ListenerExecution::SetRequestSchema(execution)))), feed))
        }
        "api/config/objectiveai_authorization/set/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::objectiveai_authorization::set::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::objectiveai_authorization::set::response_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::objectiveai_authorization::set::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::ObjectiveaiAuthorization(crate::cli::command::api::config::objectiveai_authorization::ListenerExecution::SetResponseSchema(execution)))), feed))
        }
        "api/config/openrouter_authorization/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::openrouter_authorization::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::openrouter_authorization::get::Response>(path_type);
            let execution = crate::cli::command::api::config::openrouter_authorization::get::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::OpenrouterAuthorization(crate::cli::command::api::config::openrouter_authorization::ListenerExecution::Get(execution)))), feed))
        }
        "api/config/openrouter_authorization/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::openrouter_authorization::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::openrouter_authorization::get::request_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::openrouter_authorization::get::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::OpenrouterAuthorization(crate::cli::command::api::config::openrouter_authorization::ListenerExecution::GetRequestSchema(execution)))), feed))
        }
        "api/config/openrouter_authorization/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::openrouter_authorization::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::openrouter_authorization::get::response_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::openrouter_authorization::get::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::OpenrouterAuthorization(crate::cli::command::api::config::openrouter_authorization::ListenerExecution::GetResponseSchema(execution)))), feed))
        }
        "api/config/openrouter_authorization/set" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::openrouter_authorization::set::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::openrouter_authorization::set::Response>(path_type);
            let execution = crate::cli::command::api::config::openrouter_authorization::set::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::OpenrouterAuthorization(crate::cli::command::api::config::openrouter_authorization::ListenerExecution::Set(execution)))), feed))
        }
        "api/config/openrouter_authorization/set/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::openrouter_authorization::set::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::openrouter_authorization::set::request_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::openrouter_authorization::set::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::OpenrouterAuthorization(crate::cli::command::api::config::openrouter_authorization::ListenerExecution::SetRequestSchema(execution)))), feed))
        }
        "api/config/openrouter_authorization/set/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::openrouter_authorization::set::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::openrouter_authorization::set::response_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::openrouter_authorization::set::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::OpenrouterAuthorization(crate::cli::command::api::config::openrouter_authorization::ListenerExecution::SetResponseSchema(execution)))), feed))
        }
        "api/config/user_agent/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::user_agent::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::user_agent::get::Response>(path_type);
            let execution = crate::cli::command::api::config::user_agent::get::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::UserAgent(crate::cli::command::api::config::user_agent::ListenerExecution::Get(execution)))), feed))
        }
        "api/config/user_agent/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::user_agent::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::user_agent::get::request_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::user_agent::get::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::UserAgent(crate::cli::command::api::config::user_agent::ListenerExecution::GetRequestSchema(execution)))), feed))
        }
        "api/config/user_agent/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::user_agent::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::user_agent::get::response_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::user_agent::get::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::UserAgent(crate::cli::command::api::config::user_agent::ListenerExecution::GetResponseSchema(execution)))), feed))
        }
        "api/config/user_agent/set" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::user_agent::set::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::user_agent::set::Response>(path_type);
            let execution = crate::cli::command::api::config::user_agent::set::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::UserAgent(crate::cli::command::api::config::user_agent::ListenerExecution::Set(execution)))), feed))
        }
        "api/config/user_agent/set/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::user_agent::set::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::user_agent::set::request_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::user_agent::set::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::UserAgent(crate::cli::command::api::config::user_agent::ListenerExecution::SetRequestSchema(execution)))), feed))
        }
        "api/config/user_agent/set/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::user_agent::set::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::user_agent::set::response_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::user_agent::set::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::UserAgent(crate::cli::command::api::config::user_agent::ListenerExecution::SetResponseSchema(execution)))), feed))
        }
        "api/config/x_title/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::x_title::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::x_title::get::Response>(path_type);
            let execution = crate::cli::command::api::config::x_title::get::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::XTitle(crate::cli::command::api::config::x_title::ListenerExecution::Get(execution)))), feed))
        }
        "api/config/x_title/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::x_title::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::x_title::get::request_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::x_title::get::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::XTitle(crate::cli::command::api::config::x_title::ListenerExecution::GetRequestSchema(execution)))), feed))
        }
        "api/config/x_title/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::x_title::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::x_title::get::response_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::x_title::get::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::XTitle(crate::cli::command::api::config::x_title::ListenerExecution::GetResponseSchema(execution)))), feed))
        }
        "api/config/x_title/set" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::x_title::set::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::x_title::set::Response>(path_type);
            let execution = crate::cli::command::api::config::x_title::set::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::XTitle(crate::cli::command::api::config::x_title::ListenerExecution::Set(execution)))), feed))
        }
        "api/config/x_title/set/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::x_title::set::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::x_title::set::request_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::x_title::set::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::XTitle(crate::cli::command::api::config::x_title::ListenerExecution::SetRequestSchema(execution)))), feed))
        }
        "api/config/x_title/set/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::api::config::x_title::set::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::api::config::x_title::set::response_schema::Response>(path_type);
            let execution = crate::cli::command::api::config::x_title::set::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Api(crate::cli::command::api::ListenerExecution::Config(crate::cli::command::api::config::ListenerExecution::XTitle(crate::cli::command::api::config::x_title::ListenerExecution::SetResponseSchema(execution)))), feed))
        }
        "daemon/config/address/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::daemon::config::address::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::daemon::config::address::get::Response>(path_type);
            let execution = crate::cli::command::daemon::config::address::get::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Daemon(crate::cli::command::daemon::ListenerExecution::Config(crate::cli::command::daemon::config::ListenerExecution::Address(crate::cli::command::daemon::config::address::ListenerExecution::Get(execution)))), feed))
        }
        "daemon/config/address/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::daemon::config::address::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::daemon::config::address::get::request_schema::Response>(path_type);
            let execution = crate::cli::command::daemon::config::address::get::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Daemon(crate::cli::command::daemon::ListenerExecution::Config(crate::cli::command::daemon::config::ListenerExecution::Address(crate::cli::command::daemon::config::address::ListenerExecution::GetRequestSchema(execution)))), feed))
        }
        "daemon/config/address/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::daemon::config::address::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::daemon::config::address::get::response_schema::Response>(path_type);
            let execution = crate::cli::command::daemon::config::address::get::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Daemon(crate::cli::command::daemon::ListenerExecution::Config(crate::cli::command::daemon::config::ListenerExecution::Address(crate::cli::command::daemon::config::address::ListenerExecution::GetResponseSchema(execution)))), feed))
        }
        "daemon/config/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::daemon::config::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::daemon::config::get::Response>(path_type);
            let execution = crate::cli::command::daemon::config::get::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Daemon(crate::cli::command::daemon::ListenerExecution::Config(crate::cli::command::daemon::config::ListenerExecution::Get(execution))), feed))
        }
        "daemon/config/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::daemon::config::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::daemon::config::get::request_schema::Response>(path_type);
            let execution = crate::cli::command::daemon::config::get::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Daemon(crate::cli::command::daemon::ListenerExecution::Config(crate::cli::command::daemon::config::ListenerExecution::GetRequestSchema(execution))), feed))
        }
        "daemon/config/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::daemon::config::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::daemon::config::get::response_schema::Response>(path_type);
            let execution = crate::cli::command::daemon::config::get::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Daemon(crate::cli::command::daemon::ListenerExecution::Config(crate::cli::command::daemon::config::ListenerExecution::GetResponseSchema(execution))), feed))
        }
        "daemon/config/set" => {
            let parsed = serde_json::from_str::<crate::cli::command::daemon::config::set::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::daemon::config::set::Response>(path_type);
            let execution = crate::cli::command::daemon::config::set::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Daemon(crate::cli::command::daemon::ListenerExecution::Config(crate::cli::command::daemon::config::ListenerExecution::Set(execution))), feed))
        }
        "daemon/config/set/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::daemon::config::set::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::daemon::config::set::request_schema::Response>(path_type);
            let execution = crate::cli::command::daemon::config::set::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Daemon(crate::cli::command::daemon::ListenerExecution::Config(crate::cli::command::daemon::config::ListenerExecution::SetRequestSchema(execution))), feed))
        }
        "daemon/config/set/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::daemon::config::set::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::daemon::config::set::response_schema::Response>(path_type);
            let execution = crate::cli::command::daemon::config::set::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Daemon(crate::cli::command::daemon::ListenerExecution::Config(crate::cli::command::daemon::config::ListenerExecution::SetResponseSchema(execution))), feed))
        }
        "daemon/config/refresh_secret_signature_pair" => {
            let parsed = serde_json::from_str::<crate::cli::command::daemon::config::refresh_secret_signature_pair::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::daemon::config::refresh_secret_signature_pair::Response>(path_type);
            let execution = crate::cli::command::daemon::config::refresh_secret_signature_pair::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Daemon(crate::cli::command::daemon::ListenerExecution::Config(crate::cli::command::daemon::config::ListenerExecution::RefreshSecretSignaturePair(execution))), feed))
        }
        "daemon/config/refresh_secret_signature_pair/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::daemon::config::refresh_secret_signature_pair::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::daemon::config::refresh_secret_signature_pair::request_schema::Response>(path_type);
            let execution = crate::cli::command::daemon::config::refresh_secret_signature_pair::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Daemon(crate::cli::command::daemon::ListenerExecution::Config(crate::cli::command::daemon::config::ListenerExecution::RefreshSecretSignaturePairRequestSchema(execution))), feed))
        }
        "daemon/config/refresh_secret_signature_pair/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::daemon::config::refresh_secret_signature_pair::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::daemon::config::refresh_secret_signature_pair::response_schema::Response>(path_type);
            let execution = crate::cli::command::daemon::config::refresh_secret_signature_pair::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Daemon(crate::cli::command::daemon::ListenerExecution::Config(crate::cli::command::daemon::config::ListenerExecution::RefreshSecretSignaturePairResponseSchema(execution))), feed))
        }
        "daemon/config/secret/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::daemon::config::secret::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::daemon::config::secret::get::Response>(path_type);
            let execution = crate::cli::command::daemon::config::secret::get::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Daemon(crate::cli::command::daemon::ListenerExecution::Config(crate::cli::command::daemon::config::ListenerExecution::Secret(crate::cli::command::daemon::config::secret::ListenerExecution::Get(execution)))), feed))
        }
        "daemon/config/secret/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::daemon::config::secret::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::daemon::config::secret::get::request_schema::Response>(path_type);
            let execution = crate::cli::command::daemon::config::secret::get::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Daemon(crate::cli::command::daemon::ListenerExecution::Config(crate::cli::command::daemon::config::ListenerExecution::Secret(crate::cli::command::daemon::config::secret::ListenerExecution::GetRequestSchema(execution)))), feed))
        }
        "daemon/config/secret/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::daemon::config::secret::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::daemon::config::secret::get::response_schema::Response>(path_type);
            let execution = crate::cli::command::daemon::config::secret::get::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Daemon(crate::cli::command::daemon::ListenerExecution::Config(crate::cli::command::daemon::config::ListenerExecution::Secret(crate::cli::command::daemon::config::secret::ListenerExecution::GetResponseSchema(execution)))), feed))
        }
        "daemon/config/signature/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::daemon::config::signature::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::daemon::config::signature::get::Response>(path_type);
            let execution = crate::cli::command::daemon::config::signature::get::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Daemon(crate::cli::command::daemon::ListenerExecution::Config(crate::cli::command::daemon::config::ListenerExecution::Signature(crate::cli::command::daemon::config::signature::ListenerExecution::Get(execution)))), feed))
        }
        "daemon/config/signature/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::daemon::config::signature::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::daemon::config::signature::get::request_schema::Response>(path_type);
            let execution = crate::cli::command::daemon::config::signature::get::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Daemon(crate::cli::command::daemon::ListenerExecution::Config(crate::cli::command::daemon::config::ListenerExecution::Signature(crate::cli::command::daemon::config::signature::ListenerExecution::GetRequestSchema(execution)))), feed))
        }
        "daemon/config/signature/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::daemon::config::signature::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::daemon::config::signature::get::response_schema::Response>(path_type);
            let execution = crate::cli::command::daemon::config::signature::get::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Daemon(crate::cli::command::daemon::ListenerExecution::Config(crate::cli::command::daemon::config::ListenerExecution::Signature(crate::cli::command::daemon::config::signature::ListenerExecution::GetResponseSchema(execution)))), feed))
        }
        "daemon/kill" => {
            let parsed = serde_json::from_str::<crate::cli::command::daemon::kill::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::daemon::kill::Response>(path_type);
            let execution = crate::cli::command::daemon::kill::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Daemon(crate::cli::command::daemon::ListenerExecution::Kill(execution)), feed))
        }
        "daemon/kill/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::daemon::kill::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::daemon::kill::request_schema::Response>(path_type);
            let execution = crate::cli::command::daemon::kill::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Daemon(crate::cli::command::daemon::ListenerExecution::KillRequestSchema(execution)), feed))
        }
        "daemon/kill/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::daemon::kill::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::daemon::kill::response_schema::Response>(path_type);
            let execution = crate::cli::command::daemon::kill::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Daemon(crate::cli::command::daemon::ListenerExecution::KillResponseSchema(execution)), feed))
        }
        "daemon/spawn" => {
            let parsed = serde_json::from_str::<crate::cli::command::daemon::spawn::Request>(request.get()).ok()?;
            let (response, feed) = stream_feed::<crate::cli::command::daemon::spawn::ResponseItem>();
            let execution = crate::cli::command::daemon::spawn::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Daemon(crate::cli::command::daemon::ListenerExecution::Spawn(execution)), feed))
        }
        "daemon/spawn/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::daemon::spawn::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::daemon::spawn::request_schema::Response>(path_type);
            let execution = crate::cli::command::daemon::spawn::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Daemon(crate::cli::command::daemon::ListenerExecution::SpawnRequestSchema(execution)), feed))
        }
        "daemon/spawn/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::daemon::spawn::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::daemon::spawn::response_schema::Response>(path_type);
            let execution = crate::cli::command::daemon::spawn::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Daemon(crate::cli::command::daemon::ListenerExecution::SpawnResponseSchema(execution)), feed))
        }
        "db/config/address/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::address::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::address::get::Response>(path_type);
            let execution = crate::cli::command::db::config::address::get::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Db(crate::cli::command::db::ListenerExecution::Config(crate::cli::command::db::config::ListenerExecution::Address(crate::cli::command::db::config::address::ListenerExecution::Get(execution)))), feed))
        }
        "db/config/address/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::address::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::address::get::request_schema::Response>(path_type);
            let execution = crate::cli::command::db::config::address::get::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Db(crate::cli::command::db::ListenerExecution::Config(crate::cli::command::db::config::ListenerExecution::Address(crate::cli::command::db::config::address::ListenerExecution::GetRequestSchema(execution)))), feed))
        }
        "db/config/address/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::address::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::address::get::response_schema::Response>(path_type);
            let execution = crate::cli::command::db::config::address::get::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Db(crate::cli::command::db::ListenerExecution::Config(crate::cli::command::db::config::ListenerExecution::Address(crate::cli::command::db::config::address::ListenerExecution::GetResponseSchema(execution)))), feed))
        }
        "db/config/database/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::database::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::database::get::Response>(path_type);
            let execution = crate::cli::command::db::config::database::get::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Db(crate::cli::command::db::ListenerExecution::Config(crate::cli::command::db::config::ListenerExecution::Database(crate::cli::command::db::config::database::ListenerExecution::Get(execution)))), feed))
        }
        "db/config/database/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::database::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::database::get::request_schema::Response>(path_type);
            let execution = crate::cli::command::db::config::database::get::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Db(crate::cli::command::db::ListenerExecution::Config(crate::cli::command::db::config::ListenerExecution::Database(crate::cli::command::db::config::database::ListenerExecution::GetRequestSchema(execution)))), feed))
        }
        "db/config/database/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::database::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::database::get::response_schema::Response>(path_type);
            let execution = crate::cli::command::db::config::database::get::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Db(crate::cli::command::db::ListenerExecution::Config(crate::cli::command::db::config::ListenerExecution::Database(crate::cli::command::db::config::database::ListenerExecution::GetResponseSchema(execution)))), feed))
        }
        "db/config/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::get::Response>(path_type);
            let execution = crate::cli::command::db::config::get::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Db(crate::cli::command::db::ListenerExecution::Config(crate::cli::command::db::config::ListenerExecution::Get(execution))), feed))
        }
        "db/config/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::get::request_schema::Response>(path_type);
            let execution = crate::cli::command::db::config::get::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Db(crate::cli::command::db::ListenerExecution::Config(crate::cli::command::db::config::ListenerExecution::GetRequestSchema(execution))), feed))
        }
        "db/config/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::get::response_schema::Response>(path_type);
            let execution = crate::cli::command::db::config::get::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Db(crate::cli::command::db::ListenerExecution::Config(crate::cli::command::db::config::ListenerExecution::GetResponseSchema(execution))), feed))
        }
        "db/config/set" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::set::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::set::Response>(path_type);
            let execution = crate::cli::command::db::config::set::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Db(crate::cli::command::db::ListenerExecution::Config(crate::cli::command::db::config::ListenerExecution::Set(execution))), feed))
        }
        "db/config/set/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::set::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::set::request_schema::Response>(path_type);
            let execution = crate::cli::command::db::config::set::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Db(crate::cli::command::db::ListenerExecution::Config(crate::cli::command::db::config::ListenerExecution::SetRequestSchema(execution))), feed))
        }
        "db/config/set/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::set::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::set::response_schema::Response>(path_type);
            let execution = crate::cli::command::db::config::set::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Db(crate::cli::command::db::ListenerExecution::Config(crate::cli::command::db::config::ListenerExecution::SetResponseSchema(execution))), feed))
        }
        "db/config/password/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::password::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::password::get::Response>(path_type);
            let execution = crate::cli::command::db::config::password::get::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Db(crate::cli::command::db::ListenerExecution::Config(crate::cli::command::db::config::ListenerExecution::Password(crate::cli::command::db::config::password::ListenerExecution::Get(execution)))), feed))
        }
        "db/config/password/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::password::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::password::get::request_schema::Response>(path_type);
            let execution = crate::cli::command::db::config::password::get::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Db(crate::cli::command::db::ListenerExecution::Config(crate::cli::command::db::config::ListenerExecution::Password(crate::cli::command::db::config::password::ListenerExecution::GetRequestSchema(execution)))), feed))
        }
        "db/config/password/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::password::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::password::get::response_schema::Response>(path_type);
            let execution = crate::cli::command::db::config::password::get::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Db(crate::cli::command::db::ListenerExecution::Config(crate::cli::command::db::config::ListenerExecution::Password(crate::cli::command::db::config::password::ListenerExecution::GetResponseSchema(execution)))), feed))
        }
        "db/config/user/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::user::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::user::get::Response>(path_type);
            let execution = crate::cli::command::db::config::user::get::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Db(crate::cli::command::db::ListenerExecution::Config(crate::cli::command::db::config::ListenerExecution::User(crate::cli::command::db::config::user::ListenerExecution::Get(execution)))), feed))
        }
        "db/config/user/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::user::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::user::get::request_schema::Response>(path_type);
            let execution = crate::cli::command::db::config::user::get::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Db(crate::cli::command::db::ListenerExecution::Config(crate::cli::command::db::config::ListenerExecution::User(crate::cli::command::db::config::user::ListenerExecution::GetRequestSchema(execution)))), feed))
        }
        "db/config/user/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::config::user::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::config::user::get::response_schema::Response>(path_type);
            let execution = crate::cli::command::db::config::user::get::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Db(crate::cli::command::db::ListenerExecution::Config(crate::cli::command::db::config::ListenerExecution::User(crate::cli::command::db::config::user::ListenerExecution::GetResponseSchema(execution)))), feed))
        }
        "db/query" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::query::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::query::Response>(path_type);
            let execution = crate::cli::command::db::query::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Db(crate::cli::command::db::ListenerExecution::Query(execution)), feed))
        }
        "db/query/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::query::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::query::request_schema::Response>(path_type);
            let execution = crate::cli::command::db::query::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Db(crate::cli::command::db::ListenerExecution::QueryRequestSchema(execution)), feed))
        }
        "db/query/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::db::query::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::db::query::response_schema::Response>(path_type);
            let execution = crate::cli::command::db::query::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Db(crate::cli::command::db::ListenerExecution::QueryResponseSchema(execution)), feed))
        }
        "functions/execute/standard" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::execute::standard::Request>(request.get()).ok()?;
            let (variant, feed) = if parsed.dangerous_advanced.as_ref().and_then(|a| a.stream) == Some(true) {
                let (response, feed) = stream_feed::<crate::cli::command::functions::execute::standard::ResponseItem>();
                let execution = crate::cli::command::functions::execute::standard::ListenerExecutionStreaming { request: parsed, identity, response };
                (crate::cli::command::functions::execute::standard::ListenerExecutionVariant::Streaming(execution), feed)
            } else {
                let (response, feed) = unary_feed::<crate::cli::command::functions::execute::standard::Response>(path_type);
                let execution = crate::cli::command::functions::execute::standard::ListenerExecution { request: parsed, identity, response };
                (crate::cli::command::functions::execute::standard::ListenerExecutionVariant::Execution(execution), feed)
            };
            Some((crate::cli::command::ListenerExecution::Functions(crate::cli::command::functions::ListenerExecution::Execute(crate::cli::command::functions::execute::ListenerExecution::Standard(variant))), feed))
        }
        "functions/execute/standard/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::execute::standard::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::functions::execute::standard::request_schema::Response>(path_type);
            let execution = crate::cli::command::functions::execute::standard::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Functions(crate::cli::command::functions::ListenerExecution::Execute(crate::cli::command::functions::execute::ListenerExecution::StandardRequestSchema(execution))), feed))
        }
        "functions/execute/standard/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::execute::standard::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::functions::execute::standard::response_schema::Response>(path_type);
            let execution = crate::cli::command::functions::execute::standard::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Functions(crate::cli::command::functions::ListenerExecution::Execute(crate::cli::command::functions::execute::ListenerExecution::StandardResponseSchema(execution))), feed))
        }
        "functions/execute/swiss_system" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::execute::swiss_system::Request>(request.get()).ok()?;
            let (variant, feed) = if parsed.dangerous_advanced.as_ref().and_then(|a| a.stream) == Some(true) {
                let (response, feed) = stream_feed::<crate::cli::command::functions::execute::swiss_system::ResponseItem>();
                let execution = crate::cli::command::functions::execute::swiss_system::ListenerExecutionStreaming { request: parsed, identity, response };
                (crate::cli::command::functions::execute::swiss_system::ListenerExecutionVariant::Streaming(execution), feed)
            } else {
                let (response, feed) = unary_feed::<crate::cli::command::functions::execute::swiss_system::Response>(path_type);
                let execution = crate::cli::command::functions::execute::swiss_system::ListenerExecution { request: parsed, identity, response };
                (crate::cli::command::functions::execute::swiss_system::ListenerExecutionVariant::Execution(execution), feed)
            };
            Some((crate::cli::command::ListenerExecution::Functions(crate::cli::command::functions::ListenerExecution::Execute(crate::cli::command::functions::execute::ListenerExecution::SwissSystem(variant))), feed))
        }
        "functions/execute/swiss_system/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::execute::swiss_system::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::functions::execute::swiss_system::request_schema::Response>(path_type);
            let execution = crate::cli::command::functions::execute::swiss_system::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Functions(crate::cli::command::functions::ListenerExecution::Execute(crate::cli::command::functions::execute::ListenerExecution::SwissSystemRequestSchema(execution))), feed))
        }
        "functions/execute/swiss_system/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::execute::swiss_system::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::functions::execute::swiss_system::response_schema::Response>(path_type);
            let execution = crate::cli::command::functions::execute::swiss_system::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Functions(crate::cli::command::functions::ListenerExecution::Execute(crate::cli::command::functions::execute::ListenerExecution::SwissSystemResponseSchema(execution))), feed))
        }
        "functions/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::functions::get::Response>(path_type);
            let execution = crate::cli::command::functions::get::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Functions(crate::cli::command::functions::ListenerExecution::Get(execution)), feed))
        }
        "functions/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::functions::get::request_schema::Response>(path_type);
            let execution = crate::cli::command::functions::get::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Functions(crate::cli::command::functions::ListenerExecution::GetRequestSchema(execution)), feed))
        }
        "functions/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::functions::get::response_schema::Response>(path_type);
            let execution = crate::cli::command::functions::get::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Functions(crate::cli::command::functions::ListenerExecution::GetResponseSchema(execution)), feed))
        }
        "functions/list" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::list::Request>(request.get()).ok()?;
            let (response, feed) = stream_feed::<crate::cli::command::functions::list::ResponseItem>();
            let execution = crate::cli::command::functions::list::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Functions(crate::cli::command::functions::ListenerExecution::List(execution)), feed))
        }
        "functions/list/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::list::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::functions::list::request_schema::Response>(path_type);
            let execution = crate::cli::command::functions::list::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Functions(crate::cli::command::functions::ListenerExecution::ListRequestSchema(execution)), feed))
        }
        "functions/list/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::list::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::functions::list::response_schema::Response>(path_type);
            let execution = crate::cli::command::functions::list::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Functions(crate::cli::command::functions::ListenerExecution::ListResponseSchema(execution)), feed))
        }
        "functions/profiles/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::profiles::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::functions::profiles::get::Response>(path_type);
            let execution = crate::cli::command::functions::profiles::get::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Functions(crate::cli::command::functions::ListenerExecution::Profiles(crate::cli::command::functions::profiles::ListenerExecution::Get(execution))), feed))
        }
        "functions/profiles/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::profiles::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::functions::profiles::get::request_schema::Response>(path_type);
            let execution = crate::cli::command::functions::profiles::get::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Functions(crate::cli::command::functions::ListenerExecution::Profiles(crate::cli::command::functions::profiles::ListenerExecution::GetRequestSchema(execution))), feed))
        }
        "functions/profiles/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::profiles::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::functions::profiles::get::response_schema::Response>(path_type);
            let execution = crate::cli::command::functions::profiles::get::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Functions(crate::cli::command::functions::ListenerExecution::Profiles(crate::cli::command::functions::profiles::ListenerExecution::GetResponseSchema(execution))), feed))
        }
        "functions/profiles/list" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::profiles::list::Request>(request.get()).ok()?;
            let (response, feed) = stream_feed::<crate::cli::command::functions::profiles::list::ResponseItem>();
            let execution = crate::cli::command::functions::profiles::list::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Functions(crate::cli::command::functions::ListenerExecution::Profiles(crate::cli::command::functions::profiles::ListenerExecution::List(execution))), feed))
        }
        "functions/profiles/list/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::profiles::list::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::functions::profiles::list::request_schema::Response>(path_type);
            let execution = crate::cli::command::functions::profiles::list::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Functions(crate::cli::command::functions::ListenerExecution::Profiles(crate::cli::command::functions::profiles::ListenerExecution::ListRequestSchema(execution))), feed))
        }
        "functions/profiles/list/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::profiles::list::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::functions::profiles::list::response_schema::Response>(path_type);
            let execution = crate::cli::command::functions::profiles::list::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Functions(crate::cli::command::functions::ListenerExecution::Profiles(crate::cli::command::functions::profiles::ListenerExecution::ListResponseSchema(execution))), feed))
        }
        "functions/profiles/publish" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::profiles::publish::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::functions::profiles::publish::Response>(path_type);
            let execution = crate::cli::command::functions::profiles::publish::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Functions(crate::cli::command::functions::ListenerExecution::Profiles(crate::cli::command::functions::profiles::ListenerExecution::Publish(execution))), feed))
        }
        "functions/profiles/publish/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::profiles::publish::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::functions::profiles::publish::request_schema::Response>(path_type);
            let execution = crate::cli::command::functions::profiles::publish::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Functions(crate::cli::command::functions::ListenerExecution::Profiles(crate::cli::command::functions::profiles::ListenerExecution::PublishRequestSchema(execution))), feed))
        }
        "functions/profiles/publish/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::profiles::publish::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::functions::profiles::publish::response_schema::Response>(path_type);
            let execution = crate::cli::command::functions::profiles::publish::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Functions(crate::cli::command::functions::ListenerExecution::Profiles(crate::cli::command::functions::profiles::ListenerExecution::PublishResponseSchema(execution))), feed))
        }
        "functions/publish" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::publish::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::functions::publish::Response>(path_type);
            let execution = crate::cli::command::functions::publish::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Functions(crate::cli::command::functions::ListenerExecution::Publish(execution)), feed))
        }
        "functions/publish/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::publish::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::functions::publish::request_schema::Response>(path_type);
            let execution = crate::cli::command::functions::publish::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Functions(crate::cli::command::functions::ListenerExecution::PublishRequestSchema(execution)), feed))
        }
        "functions/publish/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::functions::publish::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::functions::publish::response_schema::Response>(path_type);
            let execution = crate::cli::command::functions::publish::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Functions(crate::cli::command::functions::ListenerExecution::PublishResponseSchema(execution)), feed))
        }
        "laboratories/create" => {
            let parsed = serde_json::from_str::<crate::cli::command::laboratories::create::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::laboratories::create::Response>(path_type);
            let execution = crate::cli::command::laboratories::create::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Laboratories(crate::cli::command::laboratories::ListenerExecution::Create(execution)), feed))
        }
        "laboratories/create/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::laboratories::create::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::laboratories::create::request_schema::Response>(path_type);
            let execution = crate::cli::command::laboratories::create::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Laboratories(crate::cli::command::laboratories::ListenerExecution::CreateRequestSchema(execution)), feed))
        }
        "laboratories/create/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::laboratories::create::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::laboratories::create::response_schema::Response>(path_type);
            let execution = crate::cli::command::laboratories::create::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Laboratories(crate::cli::command::laboratories::ListenerExecution::CreateResponseSchema(execution)), feed))
        }
        "laboratories/delete" => {
            let parsed = serde_json::from_str::<crate::cli::command::laboratories::delete::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::laboratories::delete::Response>(path_type);
            let execution = crate::cli::command::laboratories::delete::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Laboratories(crate::cli::command::laboratories::ListenerExecution::Delete(execution)), feed))
        }
        "laboratories/delete/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::laboratories::delete::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::laboratories::delete::request_schema::Response>(path_type);
            let execution = crate::cli::command::laboratories::delete::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Laboratories(crate::cli::command::laboratories::ListenerExecution::DeleteRequestSchema(execution)), feed))
        }
        "laboratories/delete/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::laboratories::delete::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::laboratories::delete::response_schema::Response>(path_type);
            let execution = crate::cli::command::laboratories::delete::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Laboratories(crate::cli::command::laboratories::ListenerExecution::DeleteResponseSchema(execution)), feed))
        }
        "laboratories/list" => {
            let parsed = serde_json::from_str::<crate::cli::command::laboratories::list::Request>(request.get()).ok()?;
            let (response, feed) = stream_feed::<crate::cli::command::laboratories::list::ResponseItem>();
            let execution = crate::cli::command::laboratories::list::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Laboratories(crate::cli::command::laboratories::ListenerExecution::List(execution)), feed))
        }
        "laboratories/list/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::laboratories::list::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::laboratories::list::request_schema::Response>(path_type);
            let execution = crate::cli::command::laboratories::list::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Laboratories(crate::cli::command::laboratories::ListenerExecution::ListRequestSchema(execution)), feed))
        }
        "laboratories/list/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::laboratories::list::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::laboratories::list::response_schema::Response>(path_type);
            let execution = crate::cli::command::laboratories::list::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Laboratories(crate::cli::command::laboratories::ListenerExecution::ListResponseSchema(execution)), feed))
        }
        "laboratories/kill" => {
            let parsed = serde_json::from_str::<crate::cli::command::laboratories::kill::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::laboratories::kill::Response>(path_type);
            let execution = crate::cli::command::laboratories::kill::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Laboratories(crate::cli::command::laboratories::ListenerExecution::Kill(execution)), feed))
        }
        "laboratories/kill/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::laboratories::kill::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::laboratories::kill::request_schema::Response>(path_type);
            let execution = crate::cli::command::laboratories::kill::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Laboratories(crate::cli::command::laboratories::ListenerExecution::KillRequestSchema(execution)), feed))
        }
        "laboratories/kill/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::laboratories::kill::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::laboratories::kill::response_schema::Response>(path_type);
            let execution = crate::cli::command::laboratories::kill::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Laboratories(crate::cli::command::laboratories::ListenerExecution::KillResponseSchema(execution)), feed))
        }
        "laboratories/spawn" => {
            let parsed = serde_json::from_str::<crate::cli::command::laboratories::spawn::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::laboratories::spawn::Response>(path_type);
            let execution = crate::cli::command::laboratories::spawn::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Laboratories(crate::cli::command::laboratories::ListenerExecution::Spawn(execution)), feed))
        }
        "laboratories/spawn/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::laboratories::spawn::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::laboratories::spawn::request_schema::Response>(path_type);
            let execution = crate::cli::command::laboratories::spawn::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Laboratories(crate::cli::command::laboratories::ListenerExecution::SpawnRequestSchema(execution)), feed))
        }
        "laboratories/spawn/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::laboratories::spawn::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::laboratories::spawn::response_schema::Response>(path_type);
            let execution = crate::cli::command::laboratories::spawn::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Laboratories(crate::cli::command::laboratories::ListenerExecution::SpawnResponseSchema(execution)), feed))
        }
        "python" => {
            let parsed = serde_json::from_str::<crate::cli::command::python::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::python::Response>(path_type);
            let execution = crate::cli::command::python::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Python(execution), feed))
        }
        "python/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::python::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::python::request_schema::Response>(path_type);
            let execution = crate::cli::command::python::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::PythonRequestSchema(execution), feed))
        }
        "python/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::python::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::python::response_schema::Response>(path_type);
            let execution = crate::cli::command::python::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::PythonResponseSchema(execution), feed))
        }
        "swarms/get" => {
            let parsed = serde_json::from_str::<crate::cli::command::swarms::get::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::swarms::get::Response>(path_type);
            let execution = crate::cli::command::swarms::get::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Swarms(crate::cli::command::swarms::ListenerExecution::Get(execution)), feed))
        }
        "swarms/get/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::swarms::get::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::swarms::get::request_schema::Response>(path_type);
            let execution = crate::cli::command::swarms::get::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Swarms(crate::cli::command::swarms::ListenerExecution::GetRequestSchema(execution)), feed))
        }
        "swarms/get/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::swarms::get::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::swarms::get::response_schema::Response>(path_type);
            let execution = crate::cli::command::swarms::get::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Swarms(crate::cli::command::swarms::ListenerExecution::GetResponseSchema(execution)), feed))
        }
        "swarms/list" => {
            let parsed = serde_json::from_str::<crate::cli::command::swarms::list::Request>(request.get()).ok()?;
            let (response, feed) = stream_feed::<crate::cli::command::swarms::list::ResponseItem>();
            let execution = crate::cli::command::swarms::list::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Swarms(crate::cli::command::swarms::ListenerExecution::List(execution)), feed))
        }
        "swarms/list/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::swarms::list::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::swarms::list::request_schema::Response>(path_type);
            let execution = crate::cli::command::swarms::list::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Swarms(crate::cli::command::swarms::ListenerExecution::ListRequestSchema(execution)), feed))
        }
        "swarms/list/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::swarms::list::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::swarms::list::response_schema::Response>(path_type);
            let execution = crate::cli::command::swarms::list::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Swarms(crate::cli::command::swarms::ListenerExecution::ListResponseSchema(execution)), feed))
        }
        "swarms/publish" => {
            let parsed = serde_json::from_str::<crate::cli::command::swarms::publish::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::swarms::publish::Response>(path_type);
            let execution = crate::cli::command::swarms::publish::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Swarms(crate::cli::command::swarms::ListenerExecution::Publish(execution)), feed))
        }
        "swarms/publish/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::swarms::publish::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::swarms::publish::request_schema::Response>(path_type);
            let execution = crate::cli::command::swarms::publish::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Swarms(crate::cli::command::swarms::ListenerExecution::PublishRequestSchema(execution)), feed))
        }
        "swarms/publish/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::swarms::publish::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::swarms::publish::response_schema::Response>(path_type);
            let execution = crate::cli::command::swarms::publish::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Swarms(crate::cli::command::swarms::ListenerExecution::PublishResponseSchema(execution)), feed))
        }
        "tasks/create" => {
            let parsed = serde_json::from_str::<crate::cli::command::tasks::create::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::tasks::create::Response>(path_type);
            let execution = crate::cli::command::tasks::create::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Tasks(crate::cli::command::tasks::ListenerExecution::Create(execution)), feed))
        }
        "tasks/create/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::tasks::create::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::tasks::create::request_schema::Response>(path_type);
            let execution = crate::cli::command::tasks::create::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Tasks(crate::cli::command::tasks::ListenerExecution::CreateRequestSchema(execution)), feed))
        }
        "tasks/create/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::tasks::create::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::tasks::create::response_schema::Response>(path_type);
            let execution = crate::cli::command::tasks::create::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Tasks(crate::cli::command::tasks::ListenerExecution::CreateResponseSchema(execution)), feed))
        }
        "tasks/list" => {
            let parsed = serde_json::from_str::<crate::cli::command::tasks::list::Request>(request.get()).ok()?;
            let (response, feed) = stream_feed::<crate::cli::command::tasks::list::ResponseItem>();
            let execution = crate::cli::command::tasks::list::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Tasks(crate::cli::command::tasks::ListenerExecution::List(execution)), feed))
        }
        "tasks/list/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::tasks::list::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::tasks::list::request_schema::Response>(path_type);
            let execution = crate::cli::command::tasks::list::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Tasks(crate::cli::command::tasks::ListenerExecution::ListRequestSchema(execution)), feed))
        }
        "tasks/list/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::tasks::list::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::tasks::list::response_schema::Response>(path_type);
            let execution = crate::cli::command::tasks::list::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Tasks(crate::cli::command::tasks::ListenerExecution::ListResponseSchema(execution)), feed))
        }
        "tasks/delete" => {
            let parsed = serde_json::from_str::<crate::cli::command::tasks::delete::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::tasks::delete::Response>(path_type);
            let execution = crate::cli::command::tasks::delete::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Tasks(crate::cli::command::tasks::ListenerExecution::Delete(execution)), feed))
        }
        "tasks/delete/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::tasks::delete::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::tasks::delete::request_schema::Response>(path_type);
            let execution = crate::cli::command::tasks::delete::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Tasks(crate::cli::command::tasks::ListenerExecution::DeleteRequestSchema(execution)), feed))
        }
        "tasks/delete/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::tasks::delete::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::tasks::delete::response_schema::Response>(path_type);
            let execution = crate::cli::command::tasks::delete::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Tasks(crate::cli::command::tasks::ListenerExecution::DeleteResponseSchema(execution)), feed))
        }
        "update" => {
            let parsed = serde_json::from_str::<crate::cli::command::update::Request>(request.get()).ok()?;
            let (response, feed) = stream_feed::<crate::cli::command::update::ResponseItem>();
            let execution = crate::cli::command::update::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Update(execution), feed))
        }
        "update/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::update::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::update::request_schema::Response>(path_type);
            let execution = crate::cli::command::update::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::UpdateRequestSchema(execution), feed))
        }
        "update/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::update::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::update::response_schema::Response>(path_type);
            let execution = crate::cli::command::update::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::UpdateResponseSchema(execution), feed))
        }
        "viewer/kill" => {
            let parsed = serde_json::from_str::<crate::cli::command::viewer::kill::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::viewer::kill::Response>(path_type);
            let execution = crate::cli::command::viewer::kill::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Viewer(crate::cli::command::viewer::ListenerExecution::Kill(execution)), feed))
        }
        "viewer/kill/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::viewer::kill::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::viewer::kill::request_schema::Response>(path_type);
            let execution = crate::cli::command::viewer::kill::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Viewer(crate::cli::command::viewer::ListenerExecution::KillRequestSchema(execution)), feed))
        }
        "viewer/kill/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::viewer::kill::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::viewer::kill::response_schema::Response>(path_type);
            let execution = crate::cli::command::viewer::kill::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Viewer(crate::cli::command::viewer::ListenerExecution::KillResponseSchema(execution)), feed))
        }
        "channels/publish" => {
            let parsed = serde_json::from_str::<crate::cli::command::channels::publish::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::channels::publish::Response>(path_type);
            let execution = crate::cli::command::channels::publish::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Channels(crate::cli::command::channels::ListenerExecution::Publish(execution)), feed))
        }
        "channels/publish/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::channels::publish::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::channels::publish::request_schema::Response>(path_type);
            let execution = crate::cli::command::channels::publish::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Channels(crate::cli::command::channels::ListenerExecution::PublishRequestSchema(execution)), feed))
        }
        "channels/publish/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::channels::publish::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::channels::publish::response_schema::Response>(path_type);
            let execution = crate::cli::command::channels::publish::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Channels(crate::cli::command::channels::ListenerExecution::PublishResponseSchema(execution)), feed))
        }
        "development/plugins/mcp/create" => {
            let parsed = serde_json::from_str::<crate::cli::command::development::plugins::mcp::create::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::development::plugins::mcp::create::Response>(path_type);
            let execution = crate::cli::command::development::plugins::mcp::create::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Development(crate::cli::command::development::ListenerExecution::Plugins(crate::cli::command::development::plugins::ListenerExecution::Mcp(crate::cli::command::development::plugins::mcp::ListenerExecution::Create(execution)))), feed))
        }
        "development/plugins/mcp/create/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::development::plugins::mcp::create::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::development::plugins::mcp::create::request_schema::Response>(path_type);
            let execution = crate::cli::command::development::plugins::mcp::create::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Development(crate::cli::command::development::ListenerExecution::Plugins(crate::cli::command::development::plugins::ListenerExecution::Mcp(crate::cli::command::development::plugins::mcp::ListenerExecution::CreateRequestSchema(execution)))), feed))
        }
        "development/plugins/mcp/create/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::development::plugins::mcp::create::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::development::plugins::mcp::create::response_schema::Response>(path_type);
            let execution = crate::cli::command::development::plugins::mcp::create::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Development(crate::cli::command::development::ListenerExecution::Plugins(crate::cli::command::development::plugins::ListenerExecution::Mcp(crate::cli::command::development::plugins::mcp::ListenerExecution::CreateResponseSchema(execution)))), feed))
        }
        "development/plugins/mcp/list" => {
            let parsed = serde_json::from_str::<crate::cli::command::development::plugins::mcp::list::Request>(request.get()).ok()?;
            let (response, feed) = stream_feed::<crate::cli::command::development::plugins::mcp::list::ResponseItem>();
            let execution = crate::cli::command::development::plugins::mcp::list::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Development(crate::cli::command::development::ListenerExecution::Plugins(crate::cli::command::development::plugins::ListenerExecution::Mcp(crate::cli::command::development::plugins::mcp::ListenerExecution::List(execution)))), feed))
        }
        "development/plugins/mcp/list/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::development::plugins::mcp::list::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::development::plugins::mcp::list::request_schema::Response>(path_type);
            let execution = crate::cli::command::development::plugins::mcp::list::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Development(crate::cli::command::development::ListenerExecution::Plugins(crate::cli::command::development::plugins::ListenerExecution::Mcp(crate::cli::command::development::plugins::mcp::ListenerExecution::ListRequestSchema(execution)))), feed))
        }
        "development/plugins/mcp/list/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::development::plugins::mcp::list::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::development::plugins::mcp::list::response_schema::Response>(path_type);
            let execution = crate::cli::command::development::plugins::mcp::list::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Development(crate::cli::command::development::ListenerExecution::Plugins(crate::cli::command::development::plugins::ListenerExecution::Mcp(crate::cli::command::development::plugins::mcp::ListenerExecution::ListResponseSchema(execution)))), feed))
        }
        "development/plugins/mcp/delete" => {
            let parsed = serde_json::from_str::<crate::cli::command::development::plugins::mcp::delete::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::development::plugins::mcp::delete::Response>(path_type);
            let execution = crate::cli::command::development::plugins::mcp::delete::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Development(crate::cli::command::development::ListenerExecution::Plugins(crate::cli::command::development::plugins::ListenerExecution::Mcp(crate::cli::command::development::plugins::mcp::ListenerExecution::Delete(execution)))), feed))
        }
        "development/plugins/mcp/delete/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::development::plugins::mcp::delete::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::development::plugins::mcp::delete::request_schema::Response>(path_type);
            let execution = crate::cli::command::development::plugins::mcp::delete::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Development(crate::cli::command::development::ListenerExecution::Plugins(crate::cli::command::development::plugins::ListenerExecution::Mcp(crate::cli::command::development::plugins::mcp::ListenerExecution::DeleteRequestSchema(execution)))), feed))
        }
        "development/plugins/mcp/delete/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::development::plugins::mcp::delete::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::development::plugins::mcp::delete::response_schema::Response>(path_type);
            let execution = crate::cli::command::development::plugins::mcp::delete::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Development(crate::cli::command::development::ListenerExecution::Plugins(crate::cli::command::development::plugins::ListenerExecution::Mcp(crate::cli::command::development::plugins::mcp::ListenerExecution::DeleteResponseSchema(execution)))), feed))
        }
        "development/plugins/mcp/reset" => {
            let parsed = serde_json::from_str::<crate::cli::command::development::plugins::mcp::reset::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::development::plugins::mcp::reset::Response>(path_type);
            let execution = crate::cli::command::development::plugins::mcp::reset::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Development(crate::cli::command::development::ListenerExecution::Plugins(crate::cli::command::development::plugins::ListenerExecution::Mcp(crate::cli::command::development::plugins::mcp::ListenerExecution::Reset(execution)))), feed))
        }
        "development/plugins/mcp/reset/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::development::plugins::mcp::reset::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::development::plugins::mcp::reset::request_schema::Response>(path_type);
            let execution = crate::cli::command::development::plugins::mcp::reset::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Development(crate::cli::command::development::ListenerExecution::Plugins(crate::cli::command::development::plugins::ListenerExecution::Mcp(crate::cli::command::development::plugins::mcp::ListenerExecution::ResetRequestSchema(execution)))), feed))
        }
        "development/plugins/mcp/reset/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::development::plugins::mcp::reset::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::development::plugins::mcp::reset::response_schema::Response>(path_type);
            let execution = crate::cli::command::development::plugins::mcp::reset::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Development(crate::cli::command::development::ListenerExecution::Plugins(crate::cli::command::development::plugins::ListenerExecution::Mcp(crate::cli::command::development::plugins::mcp::ListenerExecution::ResetResponseSchema(execution)))), feed))
        }
        "development/plugins/viewer/create" => {
            let parsed = serde_json::from_str::<crate::cli::command::development::plugins::viewer::create::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::development::plugins::viewer::create::Response>(path_type);
            let execution = crate::cli::command::development::plugins::viewer::create::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Development(crate::cli::command::development::ListenerExecution::Plugins(crate::cli::command::development::plugins::ListenerExecution::Viewer(crate::cli::command::development::plugins::viewer::ListenerExecution::Create(execution)))), feed))
        }
        "development/plugins/viewer/create/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::development::plugins::viewer::create::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::development::plugins::viewer::create::request_schema::Response>(path_type);
            let execution = crate::cli::command::development::plugins::viewer::create::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Development(crate::cli::command::development::ListenerExecution::Plugins(crate::cli::command::development::plugins::ListenerExecution::Viewer(crate::cli::command::development::plugins::viewer::ListenerExecution::CreateRequestSchema(execution)))), feed))
        }
        "development/plugins/viewer/create/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::development::plugins::viewer::create::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::development::plugins::viewer::create::response_schema::Response>(path_type);
            let execution = crate::cli::command::development::plugins::viewer::create::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Development(crate::cli::command::development::ListenerExecution::Plugins(crate::cli::command::development::plugins::ListenerExecution::Viewer(crate::cli::command::development::plugins::viewer::ListenerExecution::CreateResponseSchema(execution)))), feed))
        }
        "development/plugins/viewer/list" => {
            let parsed = serde_json::from_str::<crate::cli::command::development::plugins::viewer::list::Request>(request.get()).ok()?;
            let (response, feed) = stream_feed::<crate::cli::command::development::plugins::viewer::list::ResponseItem>();
            let execution = crate::cli::command::development::plugins::viewer::list::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Development(crate::cli::command::development::ListenerExecution::Plugins(crate::cli::command::development::plugins::ListenerExecution::Viewer(crate::cli::command::development::plugins::viewer::ListenerExecution::List(execution)))), feed))
        }
        "development/plugins/viewer/list/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::development::plugins::viewer::list::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::development::plugins::viewer::list::request_schema::Response>(path_type);
            let execution = crate::cli::command::development::plugins::viewer::list::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Development(crate::cli::command::development::ListenerExecution::Plugins(crate::cli::command::development::plugins::ListenerExecution::Viewer(crate::cli::command::development::plugins::viewer::ListenerExecution::ListRequestSchema(execution)))), feed))
        }
        "development/plugins/viewer/list/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::development::plugins::viewer::list::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::development::plugins::viewer::list::response_schema::Response>(path_type);
            let execution = crate::cli::command::development::plugins::viewer::list::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Development(crate::cli::command::development::ListenerExecution::Plugins(crate::cli::command::development::plugins::ListenerExecution::Viewer(crate::cli::command::development::plugins::viewer::ListenerExecution::ListResponseSchema(execution)))), feed))
        }
        "development/plugins/viewer/delete" => {
            let parsed = serde_json::from_str::<crate::cli::command::development::plugins::viewer::delete::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::development::plugins::viewer::delete::Response>(path_type);
            let execution = crate::cli::command::development::plugins::viewer::delete::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Development(crate::cli::command::development::ListenerExecution::Plugins(crate::cli::command::development::plugins::ListenerExecution::Viewer(crate::cli::command::development::plugins::viewer::ListenerExecution::Delete(execution)))), feed))
        }
        "development/plugins/viewer/delete/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::development::plugins::viewer::delete::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::development::plugins::viewer::delete::request_schema::Response>(path_type);
            let execution = crate::cli::command::development::plugins::viewer::delete::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Development(crate::cli::command::development::ListenerExecution::Plugins(crate::cli::command::development::plugins::ListenerExecution::Viewer(crate::cli::command::development::plugins::viewer::ListenerExecution::DeleteRequestSchema(execution)))), feed))
        }
        "development/plugins/viewer/delete/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::development::plugins::viewer::delete::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::development::plugins::viewer::delete::response_schema::Response>(path_type);
            let execution = crate::cli::command::development::plugins::viewer::delete::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Development(crate::cli::command::development::ListenerExecution::Plugins(crate::cli::command::development::plugins::ListenerExecution::Viewer(crate::cli::command::development::plugins::viewer::ListenerExecution::DeleteResponseSchema(execution)))), feed))
        }
        "channels/close" => {
            let parsed = serde_json::from_str::<crate::cli::command::channels::close::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::channels::close::Response>(path_type);
            let execution = crate::cli::command::channels::close::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Channels(crate::cli::command::channels::ListenerExecution::Close(execution)), feed))
        }
        "channels/close/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::channels::close::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::channels::close::request_schema::Response>(path_type);
            let execution = crate::cli::command::channels::close::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Channels(crate::cli::command::channels::ListenerExecution::CloseRequestSchema(execution)), feed))
        }
        "channels/close/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::channels::close::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::channels::close::response_schema::Response>(path_type);
            let execution = crate::cli::command::channels::close::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Channels(crate::cli::command::channels::ListenerExecution::CloseResponseSchema(execution)), feed))
        }
        "channels/logs/request" => {
            let parsed = serde_json::from_str::<crate::cli::command::channels::logs::request::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::channels::logs::request::Response>(path_type);
            let execution = crate::cli::command::channels::logs::request::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Channels(crate::cli::command::channels::ListenerExecution::Logs(crate::cli::command::channels::logs::ListenerExecution::Request(execution))), feed))
        }
        "channels/logs/request/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::channels::logs::request::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::channels::logs::request::request_schema::Response>(path_type);
            let execution = crate::cli::command::channels::logs::request::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Channels(crate::cli::command::channels::ListenerExecution::Logs(crate::cli::command::channels::logs::ListenerExecution::RequestRequestSchema(execution))), feed))
        }
        "channels/logs/request/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::channels::logs::request::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::channels::logs::request::response_schema::Response>(path_type);
            let execution = crate::cli::command::channels::logs::request::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Channels(crate::cli::command::channels::ListenerExecution::Logs(crate::cli::command::channels::logs::ListenerExecution::RequestResponseSchema(execution))), feed))
        }
        "channels/logs/reply" => {
            let parsed = serde_json::from_str::<crate::cli::command::channels::logs::reply::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::channels::logs::reply::Response>(path_type);
            let execution = crate::cli::command::channels::logs::reply::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Channels(crate::cli::command::channels::ListenerExecution::Logs(crate::cli::command::channels::logs::ListenerExecution::Reply(execution))), feed))
        }
        "channels/logs/reply/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::channels::logs::reply::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::channels::logs::reply::request_schema::Response>(path_type);
            let execution = crate::cli::command::channels::logs::reply::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Channels(crate::cli::command::channels::ListenerExecution::Logs(crate::cli::command::channels::logs::ListenerExecution::ReplyRequestSchema(execution))), feed))
        }
        "channels/logs/reply/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::channels::logs::reply::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::channels::logs::reply::response_schema::Response>(path_type);
            let execution = crate::cli::command::channels::logs::reply::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Channels(crate::cli::command::channels::ListenerExecution::Logs(crate::cli::command::channels::logs::ListenerExecution::ReplyResponseSchema(execution))), feed))
        }
        "channels/logs/list" => {
            let parsed = serde_json::from_str::<crate::cli::command::channels::logs::list::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::channels::logs::list::Response>(path_type);
            let execution = crate::cli::command::channels::logs::list::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Channels(crate::cli::command::channels::ListenerExecution::Logs(crate::cli::command::channels::logs::ListenerExecution::List(execution))), feed))
        }
        "channels/logs/list/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::channels::logs::list::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::channels::logs::list::request_schema::Response>(path_type);
            let execution = crate::cli::command::channels::logs::list::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Channels(crate::cli::command::channels::ListenerExecution::Logs(crate::cli::command::channels::logs::ListenerExecution::ListRequestSchema(execution))), feed))
        }
        "channels/logs/list/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::channels::logs::list::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::channels::logs::list::response_schema::Response>(path_type);
            let execution = crate::cli::command::channels::logs::list::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Channels(crate::cli::command::channels::ListenerExecution::Logs(crate::cli::command::channels::logs::ListenerExecution::ListResponseSchema(execution))), feed))
        }
        "channels/logs/open" => {
            let parsed = serde_json::from_str::<crate::cli::command::channels::logs::open::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::channels::logs::open::Response>(path_type);
            let execution = crate::cli::command::channels::logs::open::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Channels(crate::cli::command::channels::ListenerExecution::Logs(crate::cli::command::channels::logs::ListenerExecution::Open(execution))), feed))
        }
        "channels/logs/open/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::channels::logs::open::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::channels::logs::open::request_schema::Response>(path_type);
            let execution = crate::cli::command::channels::logs::open::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Channels(crate::cli::command::channels::ListenerExecution::Logs(crate::cli::command::channels::logs::ListenerExecution::OpenRequestSchema(execution))), feed))
        }
        "channels/logs/open/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::channels::logs::open::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::channels::logs::open::response_schema::Response>(path_type);
            let execution = crate::cli::command::channels::logs::open::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Channels(crate::cli::command::channels::ListenerExecution::Logs(crate::cli::command::channels::logs::ListenerExecution::OpenResponseSchema(execution))), feed))
        }
        "channels/logs/subscribe" => {
            let parsed = serde_json::from_str::<crate::cli::command::channels::logs::subscribe::Request>(request.get()).ok()?;
            let (response, feed) = stream_feed::<crate::cli::command::channels::logs::subscribe::ResponseItem>();
            let execution = crate::cli::command::channels::logs::subscribe::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Channels(crate::cli::command::channels::ListenerExecution::Logs(crate::cli::command::channels::logs::ListenerExecution::Subscribe(execution))), feed))
        }
        "channels/logs/subscribe/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::channels::logs::subscribe::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::channels::logs::subscribe::request_schema::Response>(path_type);
            let execution = crate::cli::command::channels::logs::subscribe::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Channels(crate::cli::command::channels::ListenerExecution::Logs(crate::cli::command::channels::logs::ListenerExecution::SubscribeRequestSchema(execution))), feed))
        }
        "channels/logs/subscribe/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::channels::logs::subscribe::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::channels::logs::subscribe::response_schema::Response>(path_type);
            let execution = crate::cli::command::channels::logs::subscribe::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Channels(crate::cli::command::channels::ListenerExecution::Logs(crate::cli::command::channels::logs::ListenerExecution::SubscribeResponseSchema(execution))), feed))
        }
        "viewer/spawn" => {
            let parsed = serde_json::from_str::<crate::cli::command::viewer::spawn::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::viewer::spawn::Response>(path_type);
            let execution = crate::cli::command::viewer::spawn::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Viewer(crate::cli::command::viewer::ListenerExecution::Spawn(execution)), feed))
        }
        "viewer/spawn/request_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::viewer::spawn::request_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::viewer::spawn::request_schema::Response>(path_type);
            let execution = crate::cli::command::viewer::spawn::request_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Viewer(crate::cli::command::viewer::ListenerExecution::SpawnRequestSchema(execution)), feed))
        }
        "viewer/spawn/response_schema" => {
            let parsed = serde_json::from_str::<crate::cli::command::viewer::spawn::response_schema::Request>(request.get()).ok()?;
            let (response, feed) = unary_feed::<crate::cli::command::viewer::spawn::response_schema::Response>(path_type);
            let execution = crate::cli::command::viewer::spawn::response_schema::ListenerExecution { request: parsed, identity, response };
            Some((crate::cli::command::ListenerExecution::Viewer(crate::cli::command::viewer::ListenerExecution::SpawnResponseSchema(execution)), feed))
        }
        _ => None,
    }
}
