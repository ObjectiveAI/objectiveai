//! Event bus. The daemon `/listen` passthrough and the viewer-executor
//! response stream fan into the same enum; the viewer's `serve()`
//! emits each event on the Tauri channel its [`Destination`] maps to
//! (`"objectiveai"` for [`Destination::Objectiveai`], the shared
//! `"plugin"` channel for [`Destination::Plugin`]).
//!
//! The JS side does no routing beyond delivering plugin-destined
//! events to the matching plugin iframe — the destination carries the
//! plugin's full coordinates, so there is nothing to infer.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

/// Where an event is delivered: the main ObjectiveAI viewer UI, or one
/// plugin's iframe (identified by its full install coordinates).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "viewer.Destination")]
pub enum Destination {
    /// The main viewer UI.
    Objectiveai,
    /// One plugin's iframe.
    Plugin {
        owner: String,
        name: String,
        version: String,
    },
}

/// Every event the viewer emits to the JS side. Serde-tagged on
/// `type` so the JS side can pattern-match each variant.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "viewer.Event")]
pub enum Event {
    /// Host → JS. Data for the destination — today that's the daemon
    /// `/listen` passthrough (standard broadcast envelope frames),
    /// destined to the main viewer UI. Nothing is emitted to plugin
    /// destinations on this variant yet.
    #[schemars(title = "Inbound")]
    Inbound {
        destination: Destination,
        value: serde_json::Value,
    },
    /// Host → JS. One response line from a viewer-executor invocation
    /// the destination itself started, terminated by a synthetic
    /// `{"type":"end"}` line — whoever runs a request gets its own
    /// response back, main UI and plugins alike. `id` is the
    /// invocation id the CALLER minted when it posted the request:
    /// it rides every response line, so concurrent invocations from
    /// one destination demux cleanly.
    #[schemars(title = "CliCommand")]
    CliCommand {
        destination: Destination,
        id: String,
        value: serde_json::Value,
    },
}

impl Event {
    /// The event's delivery target.
    pub fn destination(&self) -> &Destination {
        match self {
            Event::Inbound { destination, .. } => destination,
            Event::CliCommand { destination, .. } => destination,
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
    fn inbound_serializes_with_tag_and_destination() {
        let e = Event::Inbound {
            destination: Destination::Objectiveai,
            value: json!({"id": "abc"}),
        };
        let s = serde_json::to_string(&e).unwrap();
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["type"], "inbound");
        assert_eq!(v["destination"], "objectiveai");
        assert_eq!(v["value"], json!({"id": "abc"}));

        let back: Event = serde_json::from_str(&s).unwrap();
        match back {
            Event::Inbound { destination, value } => {
                assert_eq!(destination, Destination::Objectiveai);
                assert_eq!(value, json!({"id": "abc"}));
            }
            _ => panic!("expected Inbound"),
        }
    }

    #[test]
    fn plugin_destination_carries_full_coordinates() {
        let e = Event::CliCommand {
            destination: Destination::Plugin {
                owner: "objectiveai".to_string(),
                name: "hello".to_string(),
                version: "0.0.1".to_string(),
            },
            id: "invocation-1".to_string(),
            value: json!({"type": "end"}),
        };
        let s = serde_json::to_string(&e).unwrap();
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["type"], "cli_command");
        assert_eq!(v["id"], "invocation-1");
        assert_eq!(v["destination"]["plugin"]["owner"], "objectiveai");
        assert_eq!(v["destination"]["plugin"]["name"], "hello");
        assert_eq!(v["destination"]["plugin"]["version"], "0.0.1");

        let back: Event = serde_json::from_str(&s).unwrap();
        assert_eq!(
            back.destination(),
            &Destination::Plugin {
                owner: "objectiveai".to_string(),
                name: "hello".to_string(),
                version: "0.0.1".to_string(),
            },
        );
    }
}
