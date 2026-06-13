//! Cross-cutting request envelope shared by every transform-capable
//! leaf `Request`.
//!
//! Serde-FLATTENED onto the leaf structs, so the wire shape is
//! identical to the old per-leaf `jq` field (a top-level `"jq"`
//! property). All of the envelope's logic is delegated to methods
//! here — argv flags ([`RequestBase::push_flags`]), clap parsing
//! ([`RequestBaseArgs`]), and the transform set/clear contract used
//! by every leaf's `execute` / `execute_transform` pair — so adding
//! a future field touches exactly this file, never the leaves.
//!
//! Carried fields: `jq` + `python` (output transforms), and the
//! optional execution caps `timeout_seconds` (`--timeout`,
//! humantime) + `max_tokens` (`--max-tokens`). The caps'
//! enforcement is leaf-specific: `db query` threads a present
//! timeout to postgres (`statement_timeout` / `lock_timeout`);
//! everywhere else both caps ride as forward-compatible envelope
//! data.

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
    /// Wall-clock execution cap, in whole seconds. Parsed from
    /// `--timeout` (humantime: `30s`, `5m`, `1h30m`), `> 0`
    /// enforced at parse time. `db query` threads it to postgres
    /// when set; omit for uncapped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub timeout_seconds: Option<u64>,
    /// Response token budget, `>= 1` (`0` is rejected at parse
    /// time — omit entirely for unlimited). Forward-compatible
    /// envelope data — no leaf enforces it yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub max_tokens: Option<u64>,
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
        if let Some(secs) = self.timeout_seconds {
            argv.push("--timeout".to_string());
            argv.push(
                humantime::format_duration(std::time::Duration::from_secs(secs)).to_string(),
            );
        }
        if let Some(n) = self.max_tokens {
            argv.push("--max-tokens".to_string());
            argv.push(n.to_string());
        }
    }

    /// Drop every TRANSFORM field — `execute`'s typed-response
    /// contract (a transform would turn the output into untyped
    /// JSON the caller's `Response` type couldn't parse).
    /// Non-transform envelope fields (`timeout_seconds`,
    /// `max_tokens`) survive.
    pub fn clear_transform(&mut self) {
        self.jq = None;
        self.python = None;
    }

    /// The active output transform, if any — the read side of the
    /// `jq` / `python` pair, resolving the python-overrides-jq rule:
    /// `python` set ⇒ `Transform::Python`, else `jq` set ⇒
    /// `Transform::Jq`, else `None`. Clones the owned filter/code.
    pub fn transform(&self) -> Option<Transform> {
        if let Some(code) = &self.python {
            Some(Transform::Python(code.clone()))
        } else {
            self.jq.as_ref().map(|filter| Transform::Jq(filter.clone()))
        }
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
    /// Wall-clock execution cap (humantime: `30s`, `5m`, `1h30m`).
    /// Omit for uncapped.
    #[arg(long, value_parser = parse_timeout_seconds)]
    pub timeout: Option<u64>,
    /// Response token budget. Must be `>= 1` if set (omit the flag
    /// entirely for unlimited).
    #[arg(long, value_parser = parse_max_tokens)]
    pub max_tokens: Option<u64>,
}

/// `--timeout` value parser: humantime → whole seconds, `> 0`.
/// Sub-second durations floor to 0 via `as_secs` and are rejected
/// with the rest — the wire unit is whole seconds.
fn parse_timeout_seconds(s: &str) -> Result<u64, String> {
    let duration = humantime::parse_duration(s).map_err(|e| e.to_string())?;
    match duration.as_secs() {
        0 => Err("must be >= 1s".to_string()),
        secs => Ok(secs),
    }
}

/// `--max-tokens` value parser: `0` would mean "no limit", which is
/// spelled by omitting the flag — reject it rather than guess.
fn parse_max_tokens(s: &str) -> Result<u64, String> {
    match s.parse::<u64>().map_err(|e| e.to_string())? {
        0 => Err("must be >= 1 (omit the flag for unlimited)".to_string()),
        n => Ok(n),
    }
}

impl From<RequestBaseArgs> for RequestBase {
    fn from(args: RequestBaseArgs) -> Self {
        Self {
            jq: args.jq,
            python: args.python,
            timeout_seconds: args.timeout,
            max_tokens: args.max_tokens,
        }
    }
}
