use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "filesystem.config.Favorite")]
pub struct Favorite {
    name: String,
    #[serde(flatten)]
    #[schemars(schema_with = "crate::flatten_schema::<crate::RemotePathCommitOptional>")]
    pub path: crate::RemotePathCommitOptional,
    note: String,
}

impl Favorite {
    pub fn new(
        name: String,
        path: crate::RemotePathCommitOptional,
        note: String,
    ) -> Result<Self, super::super::Error> {
        validate_favorite_name(&name)?;
        validate_favorite_note(&note)?;
        Ok(Self { name, path, note })
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: String) -> Result<(), super::super::Error> {
        validate_favorite_name(&name)?;
        self.name = name;
        Ok(())
    }

    pub fn get_note(&self) -> &str {
        &self.note
    }

    pub fn set_note(&mut self, note: String) -> Result<(), super::super::Error> {
        validate_favorite_note(&note)?;
        self.note = note;
        Ok(())
    }

    pub fn path(&self) -> &crate::RemotePathCommitOptional {
        &self.path
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "filesystem.config.PairFavorite")]
pub struct PairFavorite {
    name: String,
    pub function: crate::RemotePathCommitOptional,
    pub profile: crate::RemotePathCommitOptional,
    note: String,
}

impl PairFavorite {
    pub fn new(
        name: String,
        function: crate::RemotePathCommitOptional,
        profile: crate::RemotePathCommitOptional,
        note: String,
    ) -> Result<Self, super::super::Error> {
        validate_favorite_name(&name)?;
        validate_favorite_note(&note)?;
        Ok(Self { name, function, profile, note })
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: String) -> Result<(), super::super::Error> {
        validate_favorite_name(&name)?;
        self.name = name;
        Ok(())
    }

    pub fn get_note(&self) -> &str {
        &self.note
    }

    pub fn set_note(&mut self, note: String) -> Result<(), super::super::Error> {
        validate_favorite_note(&note)?;
        self.note = note;
        Ok(())
    }
}

/// Validates a favorite name: alphanumeric and dashes only, max 64 characters.
pub fn validate_favorite_name(name: &str) -> Result<(), super::super::Error> {
    if name.is_empty() || name.len() > 64 {
        return Err(super::super::Error::InvalidFavoriteName(name.to_string()));
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(super::super::Error::InvalidFavoriteName(name.to_string()));
    }
    Ok(())
}

/// Validates a favorite note: max 512 characters.
pub fn validate_favorite_note(note: &str) -> Result<(), super::super::Error> {
    if note.len() > 512 {
        return Err(super::super::Error::InvalidFavoriteNote(note.to_string()));
    }
    Ok(())
}
