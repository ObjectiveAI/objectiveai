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
        tokio::fs::write(&path, bytes)
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
    /// fills in only the fields the base is missing. One special
    /// case: `api.mcp_authorization` maps are UNIONED per key with
    /// the per-state entry overwriting the global one.
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
    tokio::fs::write(&path, bytes)
        .await
        .map_err(|e| Error::Write(path, e))?;
    Ok(())
}

/// The `--final` join: global is the base, per-state fills the
/// missing fields (recursively, object-by-object — the BASE wins on
/// conflicts), EXCEPT `api.mcp_authorization`, whose maps are
/// unioned with the per-state entries overwriting the global ones
/// per key. Performed at the JSON level so it tracks the config
/// shape automatically.
fn merge_final(
    global: super::Config,
    state: super::Config,
) -> Result<super::Config, Error> {
    let mut base = serde_json::to_value(&global).map_err(Error::Serialize)?;
    let fill = serde_json::to_value(&state).map_err(Error::Serialize)?;
    merge_missing(&mut base, &fill);
    // Special case: api.mcp_authorization — union, state wins per key.
    if let Some(state_map) = fill
        .pointer("/api/mcp_authorization")
        .and_then(|v| v.as_object())
    {
        let api = base
            .as_object_mut()
            .expect("config serializes to an object")
            .entry("api")
            .or_insert_with(|| serde_json::Value::Object(Default::default()));
        if let Some(api_obj) = api.as_object_mut() {
            let target = api_obj
                .entry("mcp_authorization")
                .or_insert_with(|| serde_json::Value::Object(Default::default()));
            if let Some(target_map) = target.as_object_mut() {
                for (k, v) in state_map {
                    target_map.insert(k.clone(), v.clone());
                }
            }
        }
    }
    serde_json::from_value(base)
        .map_err(|e| Error::Parse(std::path::PathBuf::from("<final merge>"), e))
}

/// Recursive fill-the-gaps merge: objects merge key-by-key with the
/// BASE winning on conflicts; anything non-object in the base stays.
fn merge_missing(base: &mut serde_json::Value, fill: &serde_json::Value) {
    if let (serde_json::Value::Object(b), serde_json::Value::Object(f)) =
        (base, fill)
    {
        for (k, v) in f {
            match b.get_mut(k) {
                Some(bv) => merge_missing(bv, v),
                None => {
                    b.insert(k.clone(), v.clone());
                }
            }
        }
    }
}
