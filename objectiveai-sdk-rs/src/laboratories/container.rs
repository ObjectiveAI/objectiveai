//! Container-config shapes shared across the laboratory surfaces:
//! the `laboratories create` command, the daemon's laboratories
//! listeners' records, and the host's create payloads.

use std::str::FromStr;

/// One host→container bind mount. CLI wire form is the docker-style
/// `host=…,container=…` (both keys required).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "laboratories.Mount")]
pub struct Mount {
    pub host: String,
    pub container: String,
}

impl FromStr for Mount {
    type Err = String;
    /// Parse a `--mount` arg. Accepted keys: `host`, `container` (both
    /// required). Anything else is an unknown-key error.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut host: Option<String> = None;
        let mut container: Option<String> = None;
        for (k, v) in tokenize(s)? {
            match k {
                "host" => host = Some(v.to_string()),
                "container" => container = Some(v.to_string()),
                other => return Err(format!("unknown key: {other}")),
            }
        }
        match (host, container) {
            (Some(host), Some(container)) => Ok(Mount { host, container }),
            (None, _) => Err("host is required".to_string()),
            (_, None) => Err("container is required".to_string()),
        }
    }
}

/// One environment entry. CLI wire form is `KEY=VALUE`, split on the first
/// `=` so values may contain `=`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "laboratories.EnvVar")]
pub struct EnvVar {
    pub key: String,
    pub value: String,
}

impl FromStr for EnvVar {
    type Err = String;
    /// Parse an `--env` arg. Splits on the FIRST `=`; a missing `=` is an
    /// error.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.split_once('=') {
            Some((key, value)) => Ok(EnvVar {
                key: key.to_string(),
                value: value.to_string(),
            }),
            None => Err(format!("expected KEY=VALUE, got: {s}")),
        }
    }
}

/// Split a docker-style `k=v,k=v` argument into trimmed pairs — the
/// same rules as the cli's path-ref parser.
fn tokenize(s: &str) -> Result<Vec<(&str, &str)>, String> {
    s.split(',')
        .map(|pair| {
            pair.split_once('=')
                .map(|(k, v)| (k.trim(), v.trim()))
                .ok_or_else(|| format!("expected key=value, got: {pair}"))
        })
        .collect()
}
