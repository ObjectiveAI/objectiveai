use crate::path_ref::PathRef;

/// A [`PathRef`] extended with `favorite=` support.
///
/// Parsed from `key=value,key=value` format. Accepts all [`PathRef`] keys plus:
/// - `favorite` — favorite name (mutually exclusive with remote path keys)
///
/// # Examples
///
/// ```text
/// favorite=my-resource
/// remote=github,owner=user,repository=repo
/// remote=mock,name=test
/// ```
#[derive(Clone, Debug)]
pub struct FavoriteRef(pub PathRef);

impl std::str::FromStr for FavoriteRef {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<PathRef>().map(FavoriteRef)
    }
}

impl std::fmt::Display for FavoriteRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl FavoriteRef {
    /// Resolve to a remote path. If `favorite=` is set, looks up the name in the provided favorites list.
    pub async fn resolve<F, Fut>(
        self,
        get_favorites: F,
    ) -> Result<objectiveai_sdk::RemotePathCommitOptional, crate::error::Error>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Vec<crate::filesystem::config::Favorite>>,
    {
        if let Some(fav_name) = self.0.favorite {
            let favorites = get_favorites().await;
            let fav = favorites
                .into_iter()
                .find(|f| f.get_name() == fav_name)
                .ok_or_else(|| crate::error::Error::FavoriteNotFound(fav_name))?;
            Ok(fav.path.clone())
        } else {
            self.0.resolve()
        }
    }
}
