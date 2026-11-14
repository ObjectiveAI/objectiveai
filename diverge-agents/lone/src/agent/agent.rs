//! The LoNE agent.

use diverge_sdk::shared::containers::dependencies::Template;
use diverge_sdk::shared::containers::secrets::Declaration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An agent that runs a LoNE model instead of calling an upstream
/// model: a converted bundle mounted into the container, or a
/// `lone serve` endpoint on the network.
///
/// Exactly one source. A bundle is the model on disk: a converted
/// Core ML directory produced by `lone convert`, mounted at `bundle`,
/// and every turn runs `lone harness` inside the container. An
/// `endpoint` is that same harness driven as a thin client against a
/// LoNE `serve` on the network: nothing is loaded in the container,
/// so there is no Core ML and no macOS. A LoNE bundle carries its own
/// tokenizer and chat template, so the system prompt, the sampling
/// fields and whether the model thinks are the whole of what this
/// agent can be told.
///
/// # Secrets
///
/// None: the model is on disk or on the network, so there is no
/// credential. The template this agent is made from need declare no
/// name, and a name it does declare is one the program never asks for.
///
/// # The host
///
/// A bundle turn needs Core ML and the Apple Neural Engine, so macOS.
/// An endpoint turn does not: the image installs the harness without
/// Core ML, so it builds and registers anywhere, and a Linux provider
/// reaches a LoNE host on the network. See the image's `/HARNESS.md`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, Default)]
pub struct Agent {
    /// The converted bundle's directory inside the container.
    ///
    /// Must hold the `manifest.json` `lone convert` writes. The
    /// registration checks that before the agent counts as
    /// registered; a missing bundle is refused where the caller can
    /// see why. Exactly one of `bundle` and `endpoint` is required.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bundle: Option<String>,
    /// A LoNE `serve` base URL to run turns against, instead of a
    /// bundle.
    ///
    /// `lone harness --endpoint` posts the conversation and the tool
    /// definitions to `/v1/chat/completions` there; nothing is loaded
    /// in this container, so the turn needs neither Core ML nor macOS,
    /// and the server must reach a LoNE host on the network. Exactly
    /// one of `bundle` and `endpoint` is required.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    /// The system prompt, sent as the conversation's leading message.
    ///
    /// Absent sends none, and the bundle's own template is the whole
    /// of the framing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    /// Cap on generated tokens for one turn, reasoning included.
    ///
    /// Absent is the harness's 512; the model's context clamps it
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

/// The one place this agent's turns come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source<'a> {
    /// A converted bundle mounted in the container; macOS and Core ML.
    Bundle(&'a str),
    /// A `lone serve` endpoint on the network; no Core ML in this
    /// container.
    Endpoint(&'a str),
}

/// An agent that does not name exactly one source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SourceError {
    /// Both are set, so which one runs the turn is anyone's guess.
    #[error("give exactly one of `bundle` and `endpoint`; both are set")]
    Both,
    /// Neither is set, so there is nothing to run a turn against.
    #[error("give exactly one of `bundle` and `endpoint`; neither is set")]
    Neither,
}

impl Agent {
    /// The secrets this agent takes, as the registration answers
    /// them: none, whatever the arguments.
    pub fn declaration(&self) -> Declaration {
        Declaration::new()
    }

    /// The configured source, or why there is not exactly one.
    pub fn source(&self) -> Result<Source<'_>, SourceError> {
        match (self.bundle.as_deref(), self.endpoint.as_deref()) {
            (Some(bundle), None) => Ok(Source::Bundle(bundle)),
            (None, Some(endpoint)) => Ok(Source::Endpoint(endpoint)),
            (Some(_), Some(_)) => Err(SourceError::Both),
            (None, None) => Err(SourceError::Neither),
        }
    }

    /// The `lone harness` invocation this agent's turns run: the
    /// subcommand, the one source, and only the flags the caller set.
    pub fn arguments(&self) -> Vec<String> {
        let mut arguments = vec!["harness".to_owned()];
        match self
            .source()
            .expect("registration refuses an agent that names no one source")
        {
            Source::Bundle(bundle) => {
                arguments.push("--bundle".to_owned());
                arguments.push(bundle.to_owned());
            }
            Source::Endpoint(endpoint) => {
                arguments.push("--endpoint".to_owned());
                arguments.push(endpoint.to_owned());
            }
        }
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
    fn an_endpoint_runs_the_harness_without_a_bundle() {
        let agent: Agent =
            serde_json::from_str(r#"{"endpoint": "http://lone-host:8082"}"#).unwrap();
        assert_eq!(
            agent.arguments(),
            vec!["harness", "--endpoint", "http://lone-host:8082"]
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
    fn exactly_one_source_is_required() {
        let both: Agent =
            serde_json::from_str(r#"{"bundle": "/m", "endpoint": "http://h:8082"}"#).unwrap();
        assert_eq!(both.source(), Err(SourceError::Both));
        let neither: Agent = serde_json::from_str(r#"{"temperature": 0.2}"#).unwrap();
        assert_eq!(neither.source(), Err(SourceError::Neither));
    }
}
