//! `agents list available` — async handler stub.

use crate::cli::command::IntoCommand;

pub struct Request {
    pub source: RequestSource,
}

pub enum RequestSource {
    Filesystem,
    Favorites,
    Objectiveai,
    Mock,
    All,
}

impl RequestSource {
    fn as_subcommand(&self) -> &'static str {
        match self {
            RequestSource::Filesystem => "filesystem",
            RequestSource::Favorites => "favorites",
            RequestSource::Objectiveai => "objectiveai",
            RequestSource::Mock => "mock",
            RequestSource::All => "all",
        }
    }
}

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        vec![
            "agents".to_string(),
            "list".to_string(),
            "available".to_string(),
            self.source.as_subcommand().to_string(),
        ]
    }
}

pub enum Response {
    Filesystem(Vec<crate::RemotePath>),
    Favorites(Vec<crate::filesystem::config::Favorite>),
    Objectiveai(Vec<crate::RemotePath>),
    Mock(Vec<crate::RemotePath>),
    All(Vec<crate::RemotePath>),
}
