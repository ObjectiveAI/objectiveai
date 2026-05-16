use crate::favorite_ref::FavoriteRef;

/// Agent reference — a [`FavoriteRef`] that resolves to an agent-specific type.
///
/// Parsed from `key=value,key=value` format. See [`crate::path_ref::PathRef`] for supported keys.
#[derive(Clone, Debug)]
pub struct AgentRef(pub FavoriteRef);

impl std::str::FromStr for AgentRef {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<FavoriteRef>().map(AgentRef)
    }
}

impl std::fmt::Display for AgentRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl AgentRef {
    pub async fn resolve<F, Fut>(
        self,
        get_favorites: F,
    ) -> Result<
        objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional,
        crate::error::Error,
    >
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Vec<objectiveai_sdk::filesystem::config::Favorite>>,
    {
        let path = self.0.resolve(get_favorites).await?;
        Ok(
            objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::Remote(path),
        )
    }
}
