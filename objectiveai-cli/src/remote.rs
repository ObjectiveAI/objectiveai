/// Remote source including Mock.
#[derive(Clone, Debug, clap::ValueEnum)]
pub enum Remote {
    Github,
    Filesystem,
    Mock,
}

impl Remote {
    pub fn into_path(self, owner: Option<String>, repository: Option<String>, name: Option<String>, commit: Option<String>) -> Option<objectiveai_sdk::RemotePathCommitOptional> {
        match self {
            Remote::Github => {
                Some(objectiveai_sdk::RemotePathCommitOptional::Github {
                    owner: owner?,
                    repository: repository?,
                    commit,
                })
            }
            Remote::Filesystem => {
                Some(objectiveai_sdk::RemotePathCommitOptional::Filesystem {
                    owner: owner?,
                    repository: repository?,
                    commit,
                })
            }
            Remote::Mock => {
                Some(objectiveai_sdk::RemotePathCommitOptional::Mock {
                    name: name?,
                })
            }
        }
    }
}

