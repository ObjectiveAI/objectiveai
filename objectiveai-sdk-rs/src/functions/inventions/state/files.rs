use std::collections::HashMap;

/// The set of files that represent a serialized invention state on disk.
pub(super) struct Files {
    /// `parameters.json` — always present.
    pub parameters_json: String,
    /// `function.json` — the built function definition.
    pub function_json: Option<String>,
    /// `input_schema.json` — used by AlphaScalar/AlphaVector composite states.
    pub input_schema_json: Option<String>,
    /// `ESSAY.md`
    pub essay_md: Option<String>,
    /// `ESSAY_TASKS.md`
    pub essay_tasks_md: Option<String>,
    /// `README.md`
    pub readme_md: Option<String>,
}

impl Files {
    pub const PARAMETERS_JSON: &'static str = "parameters.json";
    pub const FUNCTION_JSON: &'static str = "function.json";
    pub const INPUT_SCHEMA_JSON: &'static str = "input_schema.json";
    pub const ESSAY_MD: &'static str = "ESSAY.md";
    pub const ESSAY_TASKS_MD: &'static str = "ESSAY_TASKS.md";
    pub const README_MD: &'static str = "README.md";

    pub const fn filenames() -> &'static [&'static str] {
        &[
            Self::PARAMETERS_JSON,
            Self::FUNCTION_JSON,
            Self::INPUT_SCHEMA_JSON,
            Self::ESSAY_MD,
            Self::ESSAY_TASKS_MD,
            Self::README_MD,
        ]
    }

    pub fn from_hashmap(map: HashMap<&'static str, String>) -> Result<Self, super::error::Error> {
        let parameters_json = map.get(Self::PARAMETERS_JSON)
            .cloned()
            .ok_or(super::error::Error::MissingFile(Self::PARAMETERS_JSON))?;

        Ok(Self {
            parameters_json,
            function_json: map.get(Self::FUNCTION_JSON).cloned(),
            input_schema_json: map.get(Self::INPUT_SCHEMA_JSON).cloned(),
            essay_md: map.get(Self::ESSAY_MD).cloned(),
            essay_tasks_md: map.get(Self::ESSAY_TASKS_MD).cloned(),
            readme_md: map.get(Self::README_MD).cloned(),
        })
    }

    pub fn into_hashmap(self) -> HashMap<&'static str, String> {
        let mut map = HashMap::new();
        map.insert(Self::PARAMETERS_JSON, self.parameters_json);
        if let Some(v) = self.function_json {
            map.insert(Self::FUNCTION_JSON, v);
        }
        if let Some(v) = self.input_schema_json {
            map.insert(Self::INPUT_SCHEMA_JSON, v);
        }
        if let Some(v) = self.essay_md {
            map.insert(Self::ESSAY_MD, v);
        }
        if let Some(v) = self.essay_tasks_md {
            map.insert(Self::ESSAY_TASKS_MD, v);
        }
        if let Some(v) = self.readme_md {
            map.insert(Self::README_MD, v);
        }
        map
    }
}
