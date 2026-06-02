//! `swarms list` — async handler stub.

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
        vec!["swarms".to_string(), "list".to_string(), self.source.as_subcommand().to_string()]
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ResponseFavorite {
    pub name: String,
    #[serde(flatten)]
    pub path: crate::RemotePathCommitOptional,
    pub note: String,
}

pub enum Response {
    Filesystem(Vec<crate::RemotePath>),
    Favorites(Vec<ResponseFavorite>),
    Objectiveai(Vec<crate::RemotePath>),
    Mock(Vec<crate::RemotePath>),
    All(Vec<crate::RemotePath>),
}
