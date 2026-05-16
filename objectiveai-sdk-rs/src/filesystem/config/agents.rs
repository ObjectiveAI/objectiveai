use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "filesystem.config.AgentsConfig")]
pub struct AgentsConfig {
    #[serde(skip_serializing_if = "crate::util::vec_is_none_or_empty")]
    #[schemars(extend("omitempty" = true))]
    pub favorites: Option<Vec<super::Favorite>>,
}

impl AgentsConfig {
    pub fn is_empty(&self) -> bool {
        crate::util::vec_is_none_or_empty(&self.favorites)
    }

    pub fn is_none(this: &Option<Self>) -> bool {
        this.as_ref().is_none_or(|cfg| cfg.is_empty())
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
