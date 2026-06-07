use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct Client {
    base_dir: PathBuf,
    pub commit_author_name: String,
    pub commit_author_email: String,
    /// Lazily-initialised SQLite connection shared across clones of this
    /// `Client`. Populated on first call to `filesystem::db::connection::connection`.
    /// Wrapped in `Arc<Mutex<Option<…>>>` so every clone sees the same
    /// connection once one exists, and so init failures don't poison
    /// the slot — a failed attempt leaves the inner `Option::None`
    /// intact and later calls can retry.
    db_conn: Arc<Mutex<Option<Arc<Mutex<rusqlite::Connection>>>>>,
    /// Parallel slot for the dedicated `tags.sqlite` connection — same
    /// lazy-init semantics as `db_conn` but a separate file so the
    /// agent-tags lifecycle stays isolated from the main message-log
    /// database. Now also hosts the `prompts` table for
    /// `agents message-queue {add,list}` so the queue-list leaf can JOIN
    /// prompts â¨ tags in one query.
    tags_db_conn: Arc<Mutex<Option<Arc<Mutex<rusqlite::Connection>>>>>,
    /// Parallel slot for the dedicated `tasks.sqlite` connection.
    /// Hosts the `schedules` table for `agents tasks schedule` (and
    /// future task-runner tables). Same lazy-init / no-poison
    /// semantics as the other slots.
    tasks_db_conn: Arc<Mutex<Option<Arc<Mutex<rusqlite::Connection>>>>>,
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
                        db_conn: Arc::new(Mutex::new(None)),
                        tags_db_conn: Arc::new(Mutex::new(None)),
                        tasks_db_conn: Arc::new(Mutex::new(None)),
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
            db_conn: Arc::new(Mutex::new(None)),
            tags_db_conn: Arc::new(Mutex::new(None)),
            tasks_db_conn: Arc::new(Mutex::new(None)),
        }
    }

    pub fn base_dir(&self) -> &PathBuf {
        &self.base_dir
    }

    pub fn config_path(&self) -> PathBuf {
        self.base_dir.join("config.json")
    }

    pub fn db_path(&self) -> PathBuf {
        self.base_dir.join("db.sqlite")
    }

    /// Path to the dedicated `tags.sqlite` file under `base_dir`.
    /// Also holds the `prompts` table for `agents message-queue {add,list}`.
    pub fn tags_db_path(&self) -> PathBuf {
        self.base_dir.join("tags.sqlite")
    }

    /// Path to the dedicated `tasks.sqlite` file under `base_dir`.
    /// Hosts the `schedules` table for `agents tasks schedule`.
    pub fn tasks_db_path(&self) -> PathBuf {
        self.base_dir.join("tasks.sqlite")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.base_dir.join("logs")
    }

    /// Root for the per-agent named-pipe tree managed by cli-stream
    /// (inbound `socket` + outbound `events.sock`). Each agent's
    /// pipes live under `pipes_dir().join(agent_instance_hierarchy)/`.
    pub fn pipes_dir(&self) -> PathBuf {
        self.base_dir.join("pipes")
    }

    /// Internal accessor to the lazy-init slot. Used by
    /// `filesystem::db::connection` to open the connection on first use.
    pub(crate) fn db_conn_slot(
        &self,
    ) -> &Mutex<Option<Arc<Mutex<rusqlite::Connection>>>> {
        &self.db_conn
    }

    /// Internal accessor to the dedicated tags-db lazy-init slot. Used
    /// by `filesystem::db::tags` to open the `tags.sqlite` connection
    /// on first use. The connection is shared with `filesystem::db::prompts`
    /// since both tables live in the same file.
    pub(crate) fn tags_db_conn_slot(
        &self,
    ) -> &Mutex<Option<Arc<Mutex<rusqlite::Connection>>>> {
        &self.tags_db_conn
    }

    /// Internal accessor to the dedicated tasks-db lazy-init slot.
    /// Used by `filesystem::db::tasks` to open the `tasks.sqlite`
    /// connection on first use.
    pub(crate) fn tasks_db_conn_slot(
        &self,
    ) -> &Mutex<Option<Arc<Mutex<rusqlite::Connection>>>> {
        &self.tasks_db_conn
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
