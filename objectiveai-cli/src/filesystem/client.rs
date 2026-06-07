use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Client {
    base_dir: PathBuf,
    pub commit_author_name: String,
    pub commit_author_email: String,
}

impl Client {
    pub fn new(
        base_dir: Option<impl Into<PathBuf>>,
        commit_author_name: Option<impl Into<String>>,
        commit_author_email: Option<impl Into<String>>,
    ) -> Self {
        let base_dir = match base_dir {
            Some(dir) => dir.into(),
            None => {
                #[cfg(feature = "env")]
                if let Ok(dir) = std::env::var("CONFIG_BASE_DIR") {
                    return Self {
                        base_dir: PathBuf::from(dir),
                        commit_author_name: resolve_author_name(
                            commit_author_name,
                        ),
                        commit_author_email: resolve_author_email(
                            commit_author_email,
                        ),
                    };
                }
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".objectiveai")
            }
        };
        Self {
            base_dir,
            commit_author_name: resolve_author_name(commit_author_name),
            commit_author_email: resolve_author_email(commit_author_email),
        }
    }

    pub fn base_dir(&self) -> &PathBuf {
        &self.base_dir
    }

    pub fn config_path(&self) -> PathBuf {
        self.base_dir.join("config.json")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.base_dir.join("logs")
    }
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
