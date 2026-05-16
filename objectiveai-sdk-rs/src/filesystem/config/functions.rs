use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "filesystem.config.FunctionsConfig")]
pub struct FunctionsConfig {
    #[serde(skip_serializing_if = "FunctionsInventionsConfig::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub inventions: Option<FunctionsInventionsConfig>,
    #[serde(skip_serializing_if = "FunctionsProfilesConfig::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub profiles: Option<FunctionsProfilesConfig>,
    #[serde(skip_serializing_if = "crate::util::vec_is_none_or_empty")]
    #[schemars(extend("omitempty" = true))]
    pub favorites: Option<Vec<super::Favorite>>,
}

impl FunctionsConfig {
    pub fn is_empty(&self) -> bool {
        self.inventions.as_ref().is_none_or(|cfg| cfg.is_empty())
    }

    pub fn is_none(this: &Option<Self>) -> bool {
        this.as_ref().is_none_or(|cfg| cfg.is_empty())
    }

    pub fn inventions(&mut self) -> &mut FunctionsInventionsConfig {
        self.inventions.get_or_insert_with(FunctionsInventionsConfig::default)
    }

    pub fn profiles(&mut self) -> &mut FunctionsProfilesConfig {
        self.profiles.get_or_insert_with(FunctionsProfilesConfig::default)
    }

    pub fn get_favorites(&self) -> &[super::Favorite] {
        self.favorites.as_deref().unwrap_or(&[])
    }

    pub fn add_favorite(&mut self, favorite: super::Favorite) {
        self.favorites.get_or_insert_with(Vec::new).push(favorite);
    }

    pub fn del_favorite(&mut self, name: &str) -> Result<(), super::super::Error> {
        let favorites = self.favorites.as_mut().ok_or_else(|| super::super::Error::FavoriteNotFound(name.to_string()))?;
        let pos = favorites.iter().position(|f| f.get_name() == name)
            .ok_or_else(|| super::super::Error::FavoriteNotFound(name.to_string()))?;
        favorites.remove(pos);
        Ok(())
    }

    pub fn edit_favorite(&mut self, name: &str) -> Result<&mut super::Favorite, super::super::Error> {
        let favorites = self.favorites.as_mut().ok_or_else(|| super::super::Error::FavoriteNotFound(name.to_string()))?;
        favorites.iter_mut().find(|f| f.get_name() == name)
            .ok_or_else(|| super::super::Error::FavoriteNotFound(name.to_string()))
    }

    pub fn jq(&self, filter: &str) -> Result<Vec<serde_json::Value>, super::super::Error> {
        super::super::run_jq(self, filter)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "filesystem.config.FunctionsInventionsConfig")]
pub struct FunctionsInventionsConfig {
    #[serde(default = "FunctionsInventionsConfig::default_remote")]
    pub remote: crate::Remote,
}

impl Default for FunctionsInventionsConfig {
    fn default() -> Self {
        Self { remote: Self::default_remote() }
    }
}

impl FunctionsInventionsConfig {
    fn default_remote() -> crate::Remote {
        crate::Remote::Filesystem
    }

    pub fn is_empty(&self) -> bool {
        matches!(self.remote, crate::Remote::Filesystem)
    }

    pub fn is_none(this: &Option<Self>) -> bool {
        this.as_ref().is_none_or(|cfg| cfg.is_empty())
    }

    pub fn get_remote(&self) -> crate::Remote {
        self.remote
    }

    pub fn set_remote(&mut self, remote: crate::Remote) -> Result<(), super::super::Error> {
        if matches!(remote, crate::Remote::Mock) {
            return Err(super::super::Error::InvalidRemote(remote));
        }
        self.remote = remote;
        Ok(())
    }

    pub fn jq(&self, filter: &str) -> Result<Vec<serde_json::Value>, super::super::Error> {
        super::super::run_jq(self, filter)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "filesystem.config.FunctionsProfilesConfig")]
pub struct FunctionsProfilesConfig {
    #[serde(skip_serializing_if = "FunctionsProfilesPairsConfig::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub pairs: Option<FunctionsProfilesPairsConfig>,
    #[serde(skip_serializing_if = "crate::util::vec_is_none_or_empty")]
    #[schemars(extend("omitempty" = true))]
    pub favorites: Option<Vec<super::Favorite>>,
}

impl FunctionsProfilesConfig {
    pub fn is_empty(&self) -> bool {
        self.pairs.as_ref().is_none_or(|cfg| cfg.is_empty())
            && crate::util::vec_is_none_or_empty(&self.favorites)
    }

    pub fn is_none(this: &Option<Self>) -> bool {
        this.as_ref().is_none_or(|cfg| cfg.is_empty())
    }

    pub fn pairs(&mut self) -> &mut FunctionsProfilesPairsConfig {
        self.pairs.get_or_insert_with(FunctionsProfilesPairsConfig::default)
    }

    pub fn get_favorites(&self) -> &[super::Favorite] {
        self.favorites.as_deref().unwrap_or(&[])
    }

    pub fn add_favorite(&mut self, favorite: super::Favorite) {
        self.favorites.get_or_insert_with(Vec::new).push(favorite);
    }

    pub fn del_favorite(&mut self, name: &str) -> Result<(), super::super::Error> {
        let favorites = self.favorites.as_mut().ok_or_else(|| super::super::Error::FavoriteNotFound(name.to_string()))?;
        let pos = favorites.iter().position(|f| f.get_name() == name)
            .ok_or_else(|| super::super::Error::FavoriteNotFound(name.to_string()))?;
        favorites.remove(pos);
        Ok(())
    }

    pub fn edit_favorite(&mut self, name: &str) -> Result<&mut super::Favorite, super::super::Error> {
        let favorites = self.favorites.as_mut().ok_or_else(|| super::super::Error::FavoriteNotFound(name.to_string()))?;
        favorites.iter_mut().find(|f| f.get_name() == name)
            .ok_or_else(|| super::super::Error::FavoriteNotFound(name.to_string()))
    }

    pub fn jq(&self, filter: &str) -> Result<Vec<serde_json::Value>, super::super::Error> {
        super::super::run_jq(self, filter)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "filesystem.config.FunctionsProfilesPairsConfig")]
pub struct FunctionsProfilesPairsConfig {
    #[serde(skip_serializing_if = "crate::util::vec_is_none_or_empty")]
    #[schemars(extend("omitempty" = true))]
    pub favorites: Option<Vec<super::PairFavorite>>,
}

impl FunctionsProfilesPairsConfig {
    pub fn is_empty(&self) -> bool {
        crate::util::vec_is_none_or_empty(&self.favorites)
    }

    pub fn is_none(this: &Option<Self>) -> bool {
        this.as_ref().is_none_or(|cfg| cfg.is_empty())
    }

    pub fn get_favorites(&self) -> &[super::PairFavorite] {
        self.favorites.as_deref().unwrap_or(&[])
    }

    pub fn add_favorite(&mut self, favorite: super::PairFavorite) {
        self.favorites.get_or_insert_with(Vec::new).push(favorite);
    }

    pub fn del_favorite(&mut self, name: &str) -> Result<(), super::super::Error> {
        let favorites = self.favorites.as_mut().ok_or_else(|| super::super::Error::FavoriteNotFound(name.to_string()))?;
        let pos = favorites.iter().position(|f| f.get_name() == name)
            .ok_or_else(|| super::super::Error::FavoriteNotFound(name.to_string()))?;
        favorites.remove(pos);
        Ok(())
    }

    pub fn edit_favorite(&mut self, name: &str) -> Result<&mut super::PairFavorite, super::super::Error> {
        let favorites = self.favorites.as_mut().ok_or_else(|| super::super::Error::FavoriteNotFound(name.to_string()))?;
        favorites.iter_mut().find(|f| f.get_name() == name)
            .ok_or_else(|| super::super::Error::FavoriteNotFound(name.to_string()))
    }

    pub fn jq(&self, filter: &str) -> Result<Vec<serde_json::Value>, super::super::Error> {
        super::super::run_jq(self, filter)
    }
}
