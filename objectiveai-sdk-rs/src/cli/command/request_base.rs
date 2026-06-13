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
    /// jq filter applied to the JSON output. Ignored when `python`
    /// is also set — python overrides jq.
    pub jq: Option<String>,
    /// Python transform applied to the JSON output. Overrides `jq`
    /// when both are provided.
    pub python: Option<String>,
    // Future envelope fields land here (timeout, …) and ride every
    // delegated method below for free.
}

impl RequestBase {
    /// Append this envelope's argv flags (`--jq <filter>`,
    /// `--python <code>`, …) — the counterpart of
    /// [`RequestBaseArgs`]' parse side, called from every leaf's
    /// `into_command`.
    pub fn push_flags(&self, argv: &mut Vec<String>) {
        if let Some(jq) = &self.jq {
            argv.push("--jq".to_string());
            argv.push(jq.clone());
        }
        if let Some(python) = &self.python {
            argv.push("--python".to_string());
            argv.push(python.clone());
        }
    }

    /// Drop every TRANSFORM field — `execute`'s typed-response
    /// contract (a transform would turn the output into untyped
    /// JSON the caller's `Response` type couldn't parse).
    /// Non-transform envelope fields (a future timeout) survive.
    pub fn clear_transform(&mut self) {
        self.jq = None;
        self.python = None;
    }

    /// Install `transform`, displacing any other — `execute_transform`'s
    /// contract: exactly the requested transform is active.
    pub fn set_transform(&mut self, transform: Transform) {
        self.clear_transform();
        match transform {
            Transform::Jq(filter) => self.jq = Some(filter),
            Transform::Python(code) => self.python = Some(code),
        }
    }
}

/// Exactly one output transform. Extends by VARIANT without
/// changing any leaf's `execute_transform` signature. (A
/// hand-constructed [`RequestBase`] can carry both fields at once —
/// there, python overrides jq.)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Transform {
    /// jq filter over the JSON output.
    Jq(String),
    /// Python transform over the JSON output.
    Python(String),
}

/// Clap mirror of [`RequestBase`], `#[command(flatten)]`-ed into
/// every transform-capable leaf's `Args`.
#[derive(clap::Args)]
pub struct RequestBaseArgs {
    /// jq filter applied to the JSON output. Ignored when --python
    /// is also set — python overrides jq.
    #[arg(long)]
    pub jq: Option<String>,
    /// Python transform applied to the JSON output. Overrides --jq
    /// when both are provided.
    #[arg(long)]
    pub python: Option<String>,
}

impl From<RequestBaseArgs> for RequestBase {
    fn from(args: RequestBaseArgs) -> Self {
        Self {
            jq: args.jq,
            python: args.python,
        }
    }
}
