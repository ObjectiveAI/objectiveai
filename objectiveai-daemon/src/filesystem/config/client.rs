use super::super::{Client, Error};

impl Client {
    pub async fn read_config(&self) -> Result<super::Config, Error> {
        let path = self.config_path();
        match tokio::fs::read(&path).await {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|e| Error::Parse(path, e)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Ok(super::Config::default())
            }
            Err(e) => Err(Error::Read(path, e)),
        }
    }

    pub async fn write_config(
        &self,
        config: &super::Config,
    ) -> Result<(), Error> {
        let path = self.config_path();
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| Error::Write(parent.to_path_buf(), e))?;
        }
        let bytes =
            serde_json::to_vec_pretty(config).map_err(Error::Serialize)?;
        // Atomic replace: a cli killed mid-write must never leave a
        // truncated config.json — that would brick every later
        // command in this scope.
        crate::filesystem::util::write_atomic(&path, &bytes)
            .await
            .map_err(|e| Error::Write(path, e))?;
        Ok(())
    }

    /// Read the config file a MUTATION targets — no merging, ever:
    /// `--global` edits `<dir>/bin/config.json`, `--state` edits
    /// `<dir>/state/<state>/config.json`. Missing file = defaults.
    pub async fn read_config_at(
        &self,
        scope: objectiveai_sdk::cli::command::SetScope,
    ) -> Result<super::Config, Error> {
        let path = match scope {
            objectiveai_sdk::cli::command::SetScope::Global => self.global_config_path(),
            objectiveai_sdk::cli::command::SetScope::State => self.config_path(),
        };
        read_config_file(path).await
    }

    /// Write back the file [`Self::read_config_at`] selected.
    pub async fn write_config_at(
        &self,
        scope: objectiveai_sdk::cli::command::SetScope,
        config: &super::Config,
    ) -> Result<(), Error> {
        let path = match scope {
            objectiveai_sdk::cli::command::SetScope::Global => self.global_config_path(),
            objectiveai_sdk::cli::command::SetScope::State => self.config_path(),
        };
        write_config_file(path, config).await
    }

    /// Read the config VIEW a `get` targets: `--global` and
    /// `--state` return their file verbatim; `--final` joins them —
    /// the global (root) config is the base and the per-state config
    /// is layered on top, winning every conflict (recursively: maps
    /// like `api.mcp_authorization` union per key with the per-state
    /// entry overwriting the global one).
    pub async fn read_config_view(
        &self,
        scope: objectiveai_sdk::cli::command::GetScope,
    ) -> Result<super::Config, Error> {
        match scope {
            objectiveai_sdk::cli::command::GetScope::Global => {
                read_config_file(self.global_config_path()).await
            }
            objectiveai_sdk::cli::command::GetScope::State => {
                read_config_file(self.config_path()).await
            }
            objectiveai_sdk::cli::command::GetScope::Final => {
                let global = read_config_file(self.global_config_path()).await?;
                let state = read_config_file(self.config_path()).await?;
                merge_final(global, state)
            }
        }
    }
}

async fn read_config_file(
    path: std::path::PathBuf,
) -> Result<super::Config, Error> {
    match tokio::fs::read(&path).await {
        Ok(bytes) => {
            serde_json::from_slice(&bytes).map_err(|e| Error::Parse(path, e))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Ok(super::Config::default())
        }
        Err(e) => Err(Error::Read(path, e)),
    }
}

async fn write_config_file(
    path: std::path::PathBuf,
    config: &super::Config,
) -> Result<(), Error> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| Error::Write(parent.to_path_buf(), e))?;
    }
    let bytes = serde_json::to_vec_pretty(config).map_err(Error::Serialize)?;
    // Atomic replace — see `write_config` for the rationale.
    crate::filesystem::util::write_atomic(&path, &bytes)
        .await
        .map_err(|e| Error::Write(path, e))?;
    Ok(())
}

/// The `--final` join: the global config is the base, the per-state
/// config is layered on top and WINS every conflict. Objects merge
/// recursively (so keyed maps like `api.mcp_authorization` union per
/// key, state entry winning); any non-object state value replaces
/// the global one outright. serde's omitempty means unset state
/// fields simply don't appear, so they never clobber global values.
/// Performed at the JSON level so it tracks the config shape
/// automatically.
fn merge_final(
    global: super::Config,
    state: super::Config,
) -> Result<super::Config, Error> {
    let mut base = serde_json::to_value(&global).map_err(Error::Serialize)?;
    let overlay = serde_json::to_value(&state).map_err(Error::Serialize)?;
    merge_override(&mut base, &overlay);
    serde_json::from_value(base)
        .map_err(|e| Error::Parse(std::path::PathBuf::from("<final merge>"), e))
}

/// Recursive overlay merge: objects merge key-by-key with the
/// OVERLAY winning; everything else is replaced by the overlay.
fn merge_override(base: &mut serde_json::Value, overlay: &serde_json::Value) {
    match (&mut *base, overlay) {
        (serde_json::Value::Object(b), serde_json::Value::Object(o)) => {
            for (k, v) in o {
                match b.get_mut(k) {
                    Some(bv) => merge_override(bv, v),
                    None => {
                        b.insert(k.clone(), v.clone());
                    }
                }
            }
        }
        (b, o) => *b = o.clone(),
    }
}
