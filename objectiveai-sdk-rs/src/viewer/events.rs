//! Event bus. Built-in axum routes, dynamic plugin routes, and the
//! cli_command stream all fan into the same enum; the viewer's
//! `serve()` emits each variant as-is under the `destination` Tauri
//! channel name.
//!
//! Channel-name namespacing: `"objectiveai"` is reserved as the
//! built-in destination; plugin repositories named "objectiveai"
//! are refused at install time (see
//! `filesystem::plugins::InstallError::ReservedRepositoryName`), so
//! a plugin can't shadow built-in events.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use super::ApiCallSubType;

/// Every event the viewer emits to the JS side. Serde-tagged on
/// `type` so the JS bridge can pattern-match and decide how to
/// repackage each variant for the destination iframe.
///
/// `destination` is `"objectiveai"` for built-in events, or the
/// plugin's repository name otherwise. For `CliCommand` it's the
/// repository name of whichever iframe invoked the CLI — the bridge
/// derives it from `MessageEvent.source`, the plugin author never
/// sets it.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "viewer.Event")]
pub enum Event {
    /// Host → iframe. Carries data into the plugin (the existing
    /// path). `sub_type` is the snake_case discriminator the plugin
    /// listens on (built-ins: `agent_completions` /
    /// `functions_executions` / `functions_inventions_recursive` /
    /// `laboratories_executions`; plugins: whatever they declared in
    /// their manifest's `viewer_routes[i].type`).
    #[schemars(title = "Inbound")]
    Inbound {
        destination: String,
        sub_type: String,
        value: serde_json::Value,
    },
    /// Host → iframe. One JSON line of cli output emitted by an
    /// in-process `objectiveai_cli::run()` invocation started by
    /// this iframe via `invokeCli`. `value` is the cli's `Output<T>`
    /// envelope. No sub_type — a single invocation produces a single
    /// stream of lines.
    #[schemars(title = "CliCommand")]
    CliCommand {
        destination: String,
        value: serde_json::Value,
    },
    /// Host → iframe. One value in the response stream of an
    /// in-process upstream API call started by the iframe via
    /// `api-call-invoke`. `sub_type` identifies which endpoint the
    /// stream belongs to (lets the iframe demux multiple concurrent
    /// calls to *different* endpoints). `value` is an
    /// [`ApiCallEnvelope`](super::ApiCallEnvelope) JSON object: one
    /// `begin`, then one or more `chunk`s (or `error`), then exactly
    /// one `end`.
    #[schemars(title = "ApiCall")]
    ApiCall {
        destination: String,
        sub_type: ApiCallSubType,
        value: serde_json::Value,
    },
}

impl Event {
    /// Tauri channel the event fans out on — the repository name of
    /// the receiving iframe (or `"objectiveai"` for built-ins).
    pub fn destination(&self) -> &str {
        match self {
            Event::Inbound { destination, .. } => destination,
            Event::CliCommand { destination, .. } => destination,
            Event::ApiCall { destination, .. } => destination,
        }
    }
}

pub type EventReceiver = mpsc::UnboundedReceiver<Event>;
pub type EventSender = mpsc::UnboundedSender<Event>;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn inbound_serializes_with_tag_and_sub_type() {
        let e = Event::Inbound {
            destination: "objectiveai".to_string(),
            sub_type: "agent_completions".to_string(),
            value: json!({"id": "abc"}),
        };
        let s = serde_json::to_string(&e).unwrap();
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["type"], "inbound");
        assert_eq!(v["destination"], "objectiveai");
        assert_eq!(v["sub_type"], "agent_completions");
        assert_eq!(v["value"], json!({"id": "abc"}));

        let back: Event = serde_json::from_str(&s).unwrap();
        match back {
            Event::Inbound { destination, sub_type, value } => {
                assert_eq!(destination, "objectiveai");
                assert_eq!(sub_type, "agent_completions");
                assert_eq!(value, json!({"id": "abc"}));
            }
            _ => panic!("expected Inbound"),
        }
    }

    #[test]
    fn cli_command_serializes_with_tag_and_no_sub_type() {
        let e = Event::CliCommand {
            destination: "my_plugin".to_string(),
            value: json!({"type": "notification", "value": {"x": 1}}),
        };
        let s = serde_json::to_string(&e).unwrap();
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["type"], "cli_command");
        assert_eq!(v["destination"], "my_plugin");
        assert!(v.get("sub_type").is_none());
        assert_eq!(v["value"]["type"], "notification");
    }

    #[test]
    fn destination_accessor() {
        let i = Event::Inbound {
            destination: "d1".to_string(),
            sub_type: "s".to_string(),
            value: json!(null),
        };
        let c = Event::CliCommand {
            destination: "d2".to_string(),
            value: json!(null),
        };
        let a = Event::ApiCall {
            destination: "d3".to_string(),
            sub_type: ApiCallSubType::PostAgentCompletions,
            value: json!(null),
        };
        assert_eq!(i.destination(), "d1");
        assert_eq!(c.destination(), "d2");
        assert_eq!(a.destination(), "d3");
    }

    #[test]
    fn api_call_serializes_with_method_underscore_path_subtype() {
        let e = Event::ApiCall {
            destination: "my_plugin".to_string(),
            sub_type: ApiCallSubType::PostAgentCompletions,
            value: json!({"type": "chunk", "chunk": {"id": "abc"}}),
        };
        let s = serde_json::to_string(&e).unwrap();
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["type"], "api_call");
        assert_eq!(v["destination"], "my_plugin");
        assert_eq!(v["sub_type"], "POST_/agent/completions");
        assert_eq!(v["value"]["type"], "chunk");

        let back: Event = serde_json::from_str(&s).unwrap();
        match back {
            Event::ApiCall { destination, sub_type, value } => {
                assert_eq!(destination, "my_plugin");
                assert_eq!(sub_type, ApiCallSubType::PostAgentCompletions);
                assert_eq!(value["chunk"]["id"], "abc");
            }
            _ => panic!("expected ApiCall"),
        }
    }
}
