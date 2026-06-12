//! Cross-cutting request envelope shared by every transform-capable
//! leaf `Request`.
//!
//! Serde-FLATTENED onto the leaf structs, so the wire shape is
//! identical to the old per-leaf `jq` field (a top-level `"jq"`
//! property). All of the envelope's logic is delegated to methods
//! here — argv flags ([`RequestBase::push_flags`]), clap parsing
//! ([`RequestBaseArgs`]), and the transform set/clear contract used
//! by every leaf's `execute` / `execute_transform` pair — so adding
//! a future field (python transform, timeout, …) touches exactly
//! this file, never the leaves.

// The flattened envelope. One per transform-capable leaf `Request`,
// as `#[serde(flatten)] pub base: RequestBase`. (Deliberately NOT a
// doc comment: schemars merges a flattened struct's description into
// every parent Request schema.)
#[derive(
    Debug,
    Clone,
    Default,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[schemars(rename = "cli.command.RequestBase")]
pub struct RequestBase {
    /// jq filter applied to the JSON output.
    pub jq: Option<String>,
    // Future envelope fields land here (python transform, timeout)
    // and ride every delegated method below for free.
}

impl RequestBase {
    /// Append this envelope's argv flags (`--jq <filter>`, …) — the
    /// counterpart of [`RequestBaseArgs`]' parse side, called from
    /// every leaf's `into_command`.
    pub fn push_flags(&self, argv: &mut Vec<String>) {
        if let Some(jq) = &self.jq {
            argv.push("--jq".to_string());
            argv.push(jq.clone());
        }
    }

    /// Drop every TRANSFORM field — `execute`'s typed-response
    /// contract (a transform would turn the output into untyped
    /// JSON the caller's `Response` type couldn't parse).
    /// Non-transform envelope fields (a future timeout) survive.
    pub fn clear_transform(&mut self) {
        self.jq = None;
    }

    /// Install `transform`, displacing any other — `execute_transform`'s
    /// contract: exactly the requested transform is active.
    pub fn set_transform(&mut self, transform: Transform) {
        self.clear_transform();
        match transform {
            Transform::Jq(filter) => self.jq = Some(filter),
        }
    }
}

/// Exactly one output transform. Extends by VARIANT (python, …)
/// without changing any leaf's `execute_transform` signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Transform {
    /// jq filter over the JSON output.
    Jq(String),
}

/// Clap mirror of [`RequestBase`], `#[command(flatten)]`-ed into
/// every transform-capable leaf's `Args`.
#[derive(clap::Args)]
pub struct RequestBaseArgs {
    /// jq filter applied to the JSON output.
    #[arg(long)]
    pub jq: Option<String>,
}

impl From<RequestBaseArgs> for RequestBase {
    fn from(args: RequestBaseArgs) -> Self {
        Self { jq: args.jq }
    }
}
