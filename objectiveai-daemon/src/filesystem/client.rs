use std::path::PathBuf;

/// On-disk layout root + state selection.
///
/// ```text
/// <dir>                              default: ~/.objectiveai
/// ├── bin/                           ── shared across states ──
/// │   ├── objectiveai[.exe]          (+ -api, -viewer, -mcp, -db)
/// │   ├── pg-bin/                    postgres install (objectiveai-db)
/// │   ├── plugins/<owner>/<name>/<version>/
/// │   └── tools/<owner>/<name>/<version>/
/// └── state/<state>/                 default state: "default"
///     ├── config.json
///     ├── db/  .pgpass  db.lock      (cluster, per state)
///     ├── logs/
///     └── instances/agents/
/// ```
///
/// Binaries, the postgres install, plugins and tools are
/// machine-wide ([`Self::bin_dir`]); everything else is per-state
/// ([`Self::state_dir`]), so independent states coexist under one
/// install by switching `OBJECTIVEAI_STATE`.
#[derive(Debug, Clone)]
pub struct Client {
    dir: PathBuf,
    state: String,
    pub commit_author_name: String,
    pub commit_author_email: String,
}

impl Client {
    /// Resolution per field: explicit arg → env (`OBJECTIVEAI_DIR` /
    /// `OBJECTIVEAI_STATE`, feature `env`) → default
    /// (`~/.objectiveai` / `"default"`).
    ///
    /// Panics when the state name doesn't match `[A-Za-z0-9_-]+` —
    /// state names become directory names under `<dir>/state/`, so
    /// separators, dot-segments, and empty names are rejected
    /// outright.
    pub fn new(
        dir: Option<impl Into<PathBuf>>,
        state: Option<impl Into<String>>,
        commit_author_name: Option<impl Into<String>>,
        commit_author_email: Option<impl Into<String>>,
    ) -> Self {
        let dir = match dir {
            Some(dir) => dir.into(),
            None => {
                #[cfg(feature = "env")]
                let env_dir = std::env::var("OBJECTIVEAI_DIR").ok();
                #[cfg(not(feature = "env"))]
                let env_dir: Option<String> = None;
                match env_dir {
                    Some(dir) => PathBuf::from(dir),
                    None => dirs::home_dir()
                        .unwrap_or_else(|| PathBuf::from("."))
                        .join(".objectiveai"),
                }
            }
        };
        let state = match state {
            Some(state) => state.into(),
            None => {
                #[cfg(feature = "env")]
                let env_state = std::env::var("OBJECTIVEAI_STATE").ok();
                #[cfg(not(feature = "env"))]
                let env_state: Option<String> = None;
                env_state.unwrap_or_else(|| "default".to_string())
            }
        };
        assert!(
            is_valid_state_name(&state),
            "OBJECTIVEAI_STATE {state:?} is invalid: state names must match [A-Za-z0-9_-]+",
        );
        Self {
            dir,
            state,
            commit_author_name: resolve_author_name(commit_author_name),
            commit_author_email: resolve_author_email(commit_author_email),
        }
    }

    /// The layout root (`OBJECTIVEAI_DIR`). Forward this (with
    /// [`Self::state`]) to child processes so they resolve the same
    /// tree.
    pub fn dir(&self) -> &PathBuf {
        &self.dir
    }

    /// The state name (`OBJECTIVEAI_STATE`).
    pub fn state(&self) -> &str {
        &self.state
    }

    /// `<dir>/bin` — binaries, the postgres install, plugins, tools.
    /// Shared across every state.
    pub fn bin_dir(&self) -> PathBuf {
        self.dir.join("bin")
    }

    /// `<dir>/state/<state>` — config, database cluster, logs, agent
    /// registry. Per state.
    pub fn state_dir(&self) -> PathBuf {
        self.dir.join("state").join(&self.state)
    }

    /// Per-state config: `<dir>/state/<state>/config.json`.
    pub fn config_path(&self) -> PathBuf {
        self.state_dir().join("config.json")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.state_dir().join("logs")
    }
}

/// `[A-Za-z0-9_-]+` — the only characters a state name may carry.
fn is_valid_state_name(state: &str) -> bool {
    !state.is_empty()
        && state
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn resolve_author_name(explicit: Option<impl Into<String>>) -> String {
    if let Some(name) = explicit {
        return name.into();
    }
    #[cfg(feature = "env")]
    if let Ok(name) = std::env::var("COMMIT_AUTHOR_NAME") {
        return name;
    }
    "ObjectiveAI".to_string()
}

fn resolve_author_email(explicit: Option<impl Into<String>>) -> String {
    if let Some(email) = explicit {
        return email.into();
    }
    #[cfg(feature = "env")]
    if let Ok(email) = std::env::var("COMMIT_AUTHOR_EMAIL") {
        return email;
    }
    "admin@objectiveai.dev".to_string()
}
