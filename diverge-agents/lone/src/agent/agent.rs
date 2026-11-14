//! The LoNE agent.

use diverge_sdk::shared::containers::dependencies::Template;
use diverge_sdk::shared::containers::secrets::Declaration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An agent that runs a local LoNE bundle instead of calling an
/// upstream model.
///
/// The bundle is the model: a converted Core ML directory produced by
/// `lone convert`, mounted into the container at `bundle`. Every turn
/// runs `lone harness` against the conversation; nothing leaves the
/// container. A LoNE bundle carries its own tokenizer and chat
/// template, so the system prompt, the sampling fields and whether
/// the model thinks are the whole of what this agent can be told.
///
/// # Secrets
///
/// None: the model is on disk, so there is no credential. The
/// template this agent is made from need declare no name, and a name
/// it does declare is one the program never asks for.
///
/// # The host
///
/// `lone` targets Core ML and the Apple Neural Engine, and runs on
/// macOS only. The image therefore needs a host that provides them;
/// on a Linux provider the harness exits saying so. See the image's
/// `/HARNESS.md`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, Default)]
pub struct Agent {
    /// The converted bundle's directory inside the container.
    ///
    /// Must hold the `manifest.json` `lone convert` writes. The
    /// registration checks that before the agent counts as
    /// registered; a missing bundle is refused where the caller can
    /// see why.
    pub bundle: String,
    /// The system prompt, sent as the conversation's leading message.
    ///
    /// Absent sends none, and the bundle's own template is the whole
    /// of the framing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    /// Cap on generated tokens for one turn, reasoning included.
    ///
    /// Absent is the harness's 512; the bundle's context clamps it
    /// further when the conversation leaves less room.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_new_tokens: Option<u64>,
    /// Sampling temperature. Absent is greedy, the harness's default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    /// Nucleus sampling. Absent keeps the whole distribution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    /// Top-k sampling. Absent keeps every token.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u64>,
    /// Whether the model may think before answering. Absent is the
    /// template's own default, which is yes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking: Option<bool>,
    /// The tool containers this agent depends on, each a dependency
    /// tool template the caller deploys and serves to it as an MCP
    /// server, exactly as the SDK's [`Template`] states one: that
    /// type's JSON Schema is what `--schema` prints for this member,
    /// in `/DIVERGE.md`, and this list, unchanged, is what the
    /// registration answers — one schema, from the one type, at both.
    /// Absent is none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<Template>,
}

impl Agent {
    /// The secrets this agent takes, as the registration answers
    /// them: none, whatever the arguments.
    pub fn declaration(&self) -> Declaration {
        Declaration::new()
    }

    /// The `lone harness` invocation this agent's turns run: the
    /// subcommand, the bundle, and only the flags the caller set.
    pub fn arguments(&self) -> Vec<String> {
        let mut arguments = vec![
            "harness".to_owned(),
            "--bundle".to_owned(),
            self.bundle.clone(),
        ];
        if let Some(system) = &self.system_prompt {
            arguments.push("--system".to_owned());
            arguments.push(system.clone());
        }
        if let Some(tokens) = self.max_new_tokens {
            arguments.push("--max-new-tokens".to_owned());
            arguments.push(tokens.to_string());
        }
        if let Some(temperature) = self.temperature {
            arguments.push("--temperature".to_owned());
            arguments.push(temperature.to_string());
        }
        if let Some(top_p) = self.top_p {
            arguments.push("--top-p".to_owned());
            arguments.push(top_p.to_string());
        }
        if let Some(top_k) = self.top_k {
            arguments.push("--top-k".to_owned());
            arguments.push(top_k.to_string());
        }
        match self.thinking {
            Some(true) => arguments.push("--thinking".to_owned()),
            Some(false) => arguments.push("--no-thinking".to_owned()),
            None => {}
        }
        arguments
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bare_bundle_runs_the_harness_with_no_other_flag() {
        let agent: Agent = serde_json::from_str(r#"{"bundle": "/models/lfm3"}"#).unwrap();
        assert_eq!(
            agent.arguments(),
            vec!["harness", "--bundle", "/models/lfm3"]
        );
    }

    #[test]
    fn only_the_sampling_members_the_caller_set_become_flags() {
        let agent: Agent = serde_json::from_str(
            r#"{"bundle": "/m", "system_prompt": "be terse", "max_new_tokens": 128,
                "temperature": 0.2, "top_p": 0.9, "top_k": 40, "thinking": false}"#,
        )
        .unwrap();
        assert_eq!(
            agent.arguments(),
            vec![
                "harness",
                "--bundle",
                "/m",
                "--system",
                "be terse",
                "--max-new-tokens",
                "128",
                "--temperature",
                "0.2",
                "--top-p",
                "0.9",
                "--top-k",
                "40",
                "--no-thinking",
            ]
        );
    }

    #[test]
    fn a_bundle_is_required() {
        assert!(serde_json::from_str::<Agent>(r#"{"temperature": 0.2}"#).is_err());
    }
}
