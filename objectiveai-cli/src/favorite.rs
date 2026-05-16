use clap::Args;

#[derive(Args)]
pub struct AddFavorite {
    /// Favorite name
    #[arg(long)]
    pub name: String,
    /// Path (e.g. remote=github,owner=x,repository=y)
    #[arg(long)]
    pub path: crate::path_ref::PathRef,
    /// Note
    #[arg(long)]
    pub note: String,
}

impl AddFavorite {
    pub fn into_favorite(self) -> Result<objectiveai_sdk::filesystem::config::Favorite, crate::error::Error> {
        let path = self.path.resolve()?;
        Ok(objectiveai_sdk::filesystem::config::Favorite::new(self.name, path, self.note)?)
    }
}

#[derive(Args)]
pub struct EditFavorite {
    /// Name of the favorite to edit
    pub name: String,
    /// Set the note
    #[arg(long)]
    pub note: Option<String>,
    /// Set the commit
    #[arg(long, conflicts_with = "remove_commit")]
    pub commit: Option<String>,
    /// Remove the commit
    #[arg(long, conflicts_with = "commit")]
    pub remove_commit: bool,
}

impl EditFavorite {
    pub fn apply(self, favorite: &mut objectiveai_sdk::filesystem::config::Favorite) -> Result<(), objectiveai_sdk::filesystem::Error> {
        if let Some(note) = self.note {
            favorite.set_note(note)?;
        }
        if let Some(commit) = self.commit {
            match &mut favorite.path {
                objectiveai_sdk::RemotePathCommitOptional::Github { commit: c, .. } => *c = Some(commit),
                objectiveai_sdk::RemotePathCommitOptional::Filesystem { commit: c, .. } => *c = Some(commit),
                objectiveai_sdk::RemotePathCommitOptional::Mock { .. } => {}
            }
        } else if self.remove_commit {
            match &mut favorite.path {
                objectiveai_sdk::RemotePathCommitOptional::Github { commit, .. } => *commit = None,
                objectiveai_sdk::RemotePathCommitOptional::Filesystem { commit, .. } => *commit = None,
                objectiveai_sdk::RemotePathCommitOptional::Mock { .. } => {}
            }
        }
        Ok(())
    }
}

#[derive(Args)]
pub struct AddPairFavorite {
    /// Favorite name
    #[arg(long)]
    pub name: String,
    /// Function path (e.g. remote=github,owner=x,repository=y)
    #[arg(long)]
    pub function: crate::path_ref::PathRef,
    /// Profile path (e.g. remote=github,owner=x,repository=y)
    #[arg(long)]
    pub profile: crate::path_ref::PathRef,
    /// Note
    #[arg(long)]
    pub note: String,
}

impl AddPairFavorite {
    pub fn into_pair_favorite(self) -> Result<objectiveai_sdk::filesystem::config::PairFavorite, crate::error::Error> {
        let function = self.function.resolve()?;
        let profile = self.profile.resolve()?;
        Ok(objectiveai_sdk::filesystem::config::PairFavorite::new(self.name, function, profile, self.note)?)
    }
}

#[derive(Args)]
pub struct EditPairFavorite {
    /// Name of the favorite to edit
    pub name: String,
    /// Set the note
    #[arg(long)]
    pub note: Option<String>,
    /// Set the function commit
    #[arg(long, conflicts_with = "remove_function_commit")]
    pub function_commit: Option<String>,
    /// Remove the function commit
    #[arg(long, conflicts_with = "function_commit")]
    pub remove_function_commit: bool,
    /// Set the profile commit
    #[arg(long, conflicts_with = "remove_profile_commit")]
    pub profile_commit: Option<String>,
    /// Remove the profile commit
    #[arg(long, conflicts_with = "profile_commit")]
    pub remove_profile_commit: bool,
}

impl EditPairFavorite {
    pub fn apply(self, favorite: &mut objectiveai_sdk::filesystem::config::PairFavorite) -> Result<(), objectiveai_sdk::filesystem::Error> {
        if let Some(note) = self.note {
            favorite.set_note(note)?;
        }
        if let Some(commit) = self.function_commit {
            match &mut favorite.function {
                objectiveai_sdk::RemotePathCommitOptional::Github { commit: c, .. } => *c = Some(commit),
                objectiveai_sdk::RemotePathCommitOptional::Filesystem { commit: c, .. } => *c = Some(commit),
                objectiveai_sdk::RemotePathCommitOptional::Mock { .. } => {}
            }
        } else if self.remove_function_commit {
            match &mut favorite.function {
                objectiveai_sdk::RemotePathCommitOptional::Github { commit, .. } => *commit = None,
                objectiveai_sdk::RemotePathCommitOptional::Filesystem { commit, .. } => *commit = None,
                objectiveai_sdk::RemotePathCommitOptional::Mock { .. } => {}
            }
        }
        if let Some(commit) = self.profile_commit {
            match &mut favorite.profile {
                objectiveai_sdk::RemotePathCommitOptional::Github { commit: c, .. } => *c = Some(commit),
                objectiveai_sdk::RemotePathCommitOptional::Filesystem { commit: c, .. } => *c = Some(commit),
                objectiveai_sdk::RemotePathCommitOptional::Mock { .. } => {}
            }
        } else if self.remove_profile_commit {
            match &mut favorite.profile {
                objectiveai_sdk::RemotePathCommitOptional::Github { commit, .. } => *commit = None,
                objectiveai_sdk::RemotePathCommitOptional::Filesystem { commit, .. } => *commit = None,
                objectiveai_sdk::RemotePathCommitOptional::Mock { .. } => {}
            }
        }
        Ok(())
    }
}
