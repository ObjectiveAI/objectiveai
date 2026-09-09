//! The caller's plugins, installed into the image's Node project.
//!
//! Everything the agent's `plugins` names is installed at `POST /run`,
//! before the runtime starts, with the command elizaOS's own installer
//! runs: `bun add --ignore-scripts <spec>` into the project at
//! [`PROJECT`]. The install is the one thing a run does before the
//! proxy is touched, and it is not the proxy's — it is the registry's,
//! over the network the container has. The version the registry
//! resolved is read back from the installed package's manifest and
//! recorded in the lineage's row; a later run of the lineage installs
//! `name@<that version>`, whatever the agent's spec said, so a
//! conversation is never resumed against a plugin that changed under
//! it. Installs are cached for the program's life by spec: a second
//! run with the same plugin set installs nothing.

use std::collections::BTreeSet;
use std::io;
use std::process::ExitStatus;
use std::sync::Mutex;

use tokio::process::Command;

use crate::agent::Plugin;
use crate::lineage::Resolved;

/// The Node project: the image's pinned plugins, the entry, and
/// everything installed here.
pub const PROJECT: &str = "/opt/diverge/eliza";

/// The specs installed this program's life.
static INSTALLED: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());

/// Install what the agent names — at the lineage's pinned version
/// where it has one — and answer each package with its resolved
/// version, in the agent's order.
pub async fn ensure(plugins: &[Plugin], pinned: &[Resolved]) -> Result<Vec<Resolved>, Error> {
    let mut resolved = Vec::with_capacity(plugins.len());
    for plugin in plugins {
        let name = package_name(&plugin.package)?;
        let spec = match pinned.iter().find(|held| held.package == name) {
            Some(held) => format!("{name}@{}", held.version),
            None => plugin.package.clone(),
        };
        if !installed(&spec) {
            install(&spec).await?;
            remember(spec);
        }
        resolved.push(Resolved {
            package: name.to_string(),
            version: version(name).await?,
        });
    }
    Ok(resolved)
}

fn installed(spec: &str) -> bool {
    INSTALLED
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .contains(spec)
}

fn remember(spec: String) {
    INSTALLED
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(spec);
}

/// The package name of a spec: the spec up to its version's `@`, the
/// scope's own `@` excepted. Validated against the installer's
/// pattern — an optional `@scope/`, then a name, each starting with
/// an alphanumeric and continuing with word characters, `.` and `-`.
pub fn package_name(spec: &str) -> Result<&str, Error> {
    let name = match spec.strip_prefix('@') {
        Some(rest) => match rest.find('@') {
            Some(at) => &spec[..at + 1],
            None => spec,
        },
        None => match spec.find('@') {
            Some(at) => &spec[..at],
            None => spec,
        },
    };
    let valid = match name.strip_prefix('@') {
        Some(scoped) => match scoped.split_once('/') {
            Some((scope, bare)) => segment(scope) && segment(bare),
            None => false,
        },
        None => segment(name),
    };
    if valid {
        Ok(name)
    } else {
        Err(Error::Name(spec.to_string()))
    }
}

/// One name segment: `[a-zA-Z0-9][\w.-]*`.
fn segment(segment: &str) -> bool {
    let mut chars = segment.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphanumeric() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}

/// `bun add --ignore-scripts <spec>` in the project.
async fn install(spec: &str) -> Result<(), Error> {
    let output = Command::new("bun")
        .arg("add")
        .arg("--ignore-scripts")
        .arg(spec)
        .current_dir(PROJECT)
        .output()
        .await
        .map_err(Error::Spawn)?;
    if output.status.success() {
        Ok(())
    } else {
        Err(Error::Failed {
            spec: spec.to_string(),
            status: output.status,
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

/// The installed package's version, from its manifest.
async fn version(name: &str) -> Result<String, Error> {
    let path = format!("{PROJECT}/node_modules/{name}/package.json");
    let manifest = tokio::fs::read(&path).await.map_err(|error| Error::Version {
        name: name.to_string(),
        error: error.to_string(),
    })?;
    let manifest: serde_json::Value =
        serde_json::from_slice(&manifest).map_err(|error| Error::Version {
            name: name.to_string(),
            error: error.to_string(),
        })?;
    manifest
        .get("version")
        .and_then(|version| version.as_str())
        .map(str::to_string)
        .ok_or_else(|| Error::Version {
            name: name.to_string(),
            error: "the manifest names no version".to_string(),
        })
}

/// A plugin could not be installed.
#[derive(Debug)]
pub enum Error {
    /// The spec is not a package name the installer takes.
    Name(String),
    /// `bun` could not be run.
    Spawn(io::Error),
    /// `bun add` failed.
    Failed {
        spec: String,
        status: ExitStatus,
        stderr: String,
    },
    /// The installed package's version could not be read.
    Version { name: String, error: String },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Name(spec) => write!(f, "{spec} is not a package spec"),
            Error::Spawn(error) => write!(f, "bun could not be run: {error}"),
            Error::Failed {
                spec,
                status,
                stderr,
            } => write!(f, "bun add {spec} failed ({status}): {stderr}"),
            Error::Version { name, error } => {
                write!(f, "the installed {name}'s version could not be read: {error}")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Spawn(error) => Some(error),
            Error::Name(_) | Error::Failed { .. } | Error::Version { .. } => None,
        }
    }
}

impl Error {
    /// The failure as JSON, the shape every failure on the stream has.
    pub fn message(&self) -> serde_json::Value {
        serde_json::json!({
            "kind": "install",
            "error": self.to_string(),
        })
    }
}
