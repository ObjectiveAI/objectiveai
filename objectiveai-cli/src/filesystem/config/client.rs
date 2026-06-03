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
}
