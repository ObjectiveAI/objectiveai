//! `swarms list` — async handler stub.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub source: RequestSource,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
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

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        vec!["swarms".to_string(), "list".to_string(), self.source.as_subcommand().to_string()]
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct ResponseFavorite {
    pub name: String,
    #[serde(flatten)]
    pub path: crate::RemotePathCommitOptional,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum Response {
    Favorites(Vec<ResponseFavorite>),
    Paths(Vec<crate::RemotePath>),
}

pub mod request_schema {
    use crate::cli::command::CommandRequest;

    pub struct Request;

    impl CommandRequest for Request {
        fn into_command(&self) -> Vec<String> {
            vec!["swarms", "list", "--request-schema"].into_iter().map(String::from).collect()
        }
    }

    pub type Response = schemars::Schema;
}


pub mod response_schema {
    use crate::cli::command::CommandRequest;

    pub struct Request;

    impl CommandRequest for Request {
        fn into_command(&self) -> Vec<String> {
            vec!["swarms", "list", "--response-schema"].into_iter().map(String::from).collect()
        }
    }

    pub type Response = schemars::Schema;
}
