mod alpha_scalar_branch_state;
mod alpha_scalar_leaf_state;
mod alpha_scalar_state;
mod alpha_vector_branch_state;
mod alpha_vector_leaf_state;
mod alpha_vector_state;
pub mod error;
pub(super) mod files;
#[cfg(feature = "http")]
mod http;

#[cfg(feature = "http")]
pub use http::*;
mod input_schema;
mod params;
mod params_state_or_remote;
mod readme;
pub mod response;

pub use input_schema::*;
pub use params_state_or_remote::*;
pub use alpha_scalar_branch_state::*;
pub use alpha_scalar_leaf_state::*;
pub use alpha_scalar_state::*;
pub use alpha_vector_branch_state::*;
pub use alpha_vector_leaf_state::*;
pub use alpha_vector_state::*;
pub use params::*;

use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// Constructs a child name by appending the task index to the parent's path.
///
/// Splits `parent_name` by `-`, takes the last segment, and tries to decode
/// it as a base62 path. If successful, pushes `task_index` and re-encodes.
/// If not, appends a new `-` segment with just the index encoded.
///
/// Detection of "is this trailing segment a path?" requires BOTH:
///   1. `last.len() == super::path::PATH_SUFFIX_LEN`, AND
///   2. `b62_to_path(last)` succeeds.
///
/// The length check is what stops a user-chosen suffix like `-v`,
/// `-vii`, or `-final` from being mistakenly decoded as a path and
/// replaced. Both checks are required — see the path module's header
/// comment for the full invariant.
fn child_name(parent_name: &str, task_index: usize) -> String {
    if let Some((prefix, last)) = parent_name.rsplit_once('-') {
        // ADDITIONAL length check — the original code only checked
        // "does it parse?", which lets `-v` (decodes as 57 → path
        // [56]) sneak through. Real paths are always exactly
        // PATH_SUFFIX_LEN base62 chars.
        if last.len() == super::path::PATH_SUFFIX_LEN {
            if let Ok(mut path) = super::path::b62_to_path::<u64>(last) {
                path.push(task_index as u64);
                if let Ok(b62) = super::path::path_to_b62(&path) {
                    return format!("{}-{}", prefix, b62);
                }
            }
        }
    }
    // Couldn't parse existing path segment — start a new one.
    let path = [task_index as u64];
    let b62 = super::path::path_to_b62(&path).unwrap_or_else(|_| format!("{}", task_index));
    format!("{}-{}", parent_name, b62)
}

/// Fixes a task name after reindexing (e.g. after a delete).
///
/// Tries to parse the last `-` segment as a base62 path. If successful,
/// pops the last element and pushes `new_index`. If parsing fails, leaves
/// the name unchanged.
///
/// Same length+parse detection rule as [`child_name`] — see its docs.
fn reindex_name(name: &mut String, new_index: usize) {
    if let Some((prefix, last)) = name.rsplit_once('-') {
        // ADDITIONAL length check — see child_name.
        if last.len() == super::path::PATH_SUFFIX_LEN {
            if let Ok(mut path) = super::path::b62_to_path::<u64>(last) {
                if !path.is_empty() {
                    path.pop();
                    path.push(new_index as u64);
                    if let Ok(b62) = super::path::path_to_b62(&path) {
                        *name = format!("{}-{}", prefix, b62);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod child_name_tests {
    use super::*;

    #[test]
    fn user_suffix_v_is_preserved() {
        // Regression: `unsettlingness-ranker-v` had its `-v` decoded
        // as base62 → integer → path and replaced with the child's
        // path. Detection now requires the trailing segment to be
        // exactly PATH_SUFFIX_LEN chars, so `"v"` (1 char) doesn't
        // qualify and a fresh path segment is appended.
        let child = child_name("unsettlingness-ranker-v", 0);
        assert!(
            child.starts_with("unsettlingness-ranker-v-"),
            "user suffix `-v` was dropped: {child:?}",
        );
    }

    #[test]
    fn user_suffix_vi_is_preserved() {
        let child = child_name("unsettlingness-ranker-vi", 0);
        assert!(
            child.starts_with("unsettlingness-ranker-vi-"),
            "user suffix `-vi` was dropped: {child:?}",
        );
    }

    #[test]
    fn user_suffix_vii_is_preserved() {
        let child = child_name("unsettlingness-ranker-vii", 0);
        assert!(
            child.starts_with("unsettlingness-ranker-vii-"),
            "user suffix `-vii` was dropped: {child:?}",
        );
    }

    #[test]
    fn other_short_user_suffixes_are_preserved() {
        for parent in ["myfn-final", "scorer-x", "cluster-abc", "thing-abcde"] {
            let child = child_name(parent, 0);
            assert!(
                child.starts_with(&format!("{parent}-")),
                "{parent:?} → {child:?} dropped the user suffix",
            );
        }
    }

    #[test]
    fn real_path_suffix_is_extended() {
        // Parents whose trailing segment IS exactly PATH_SUFFIX_LEN
        // base62 chars (and decodes as a path) get extended in place.
        // Construct a parent with such a suffix synthetically.
        let mut path = vec![3u64];
        let b62 = super::super::path::path_to_b62(&path).unwrap();
        // Pad the encoded value with arbitrary base62 chars so the
        // suffix lands at exactly PATH_SUFFIX_LEN, then re-decode to
        // discover what path that string represents — that's the
        // path child_name will see and extend.
        let padded = format!(
            "{:0>width$}",
            b62,
            width = super::super::path::PATH_SUFFIX_LEN
        );
        let parent = format!("myfn-{padded}");
        let decoded: Vec<u64> = super::super::path::b62_to_path(&padded).unwrap();
        path = decoded;
        let child = child_name(&parent, 7);
        let suffix = child.strip_prefix("myfn-").expect("user half survives");
        let extended: Vec<u64> = super::super::path::b62_to_path(suffix).unwrap();
        let mut expected = path.clone();
        expected.push(7);
        assert_eq!(extended, expected);
    }

    #[test]
    fn reindex_leaves_user_suffix_alone() {
        for original in ["scorer-v", "ranker-vi", "thing-final"] {
            let mut name = String::from(original);
            reindex_name(&mut name, 9);
            assert_eq!(name, original);
        }
    }
}

/// Abstracts over the 4 routed state variants for invention step orchestration.
pub trait InventionState: Clone + Send + 'static {
    fn params(this: &Arc<Mutex<Self>>) -> Params;
    fn is_scalar() -> bool;
    fn prompt_type() -> super::prompts::StepPromptType;
    fn object() -> super::response::streaming::Object;
    fn into_state(self) -> State;

    fn set_tasks_length(this: &Arc<Mutex<Self>>, len: u64);
    fn input_schema_json(this: &Arc<Mutex<Self>>) -> Option<String>;

    fn essay_tools(this: &Arc<Mutex<Self>>) -> Vec<super::InventionTool>;
    fn essay_tool_names(this: &Arc<Mutex<Self>>) -> Vec<String> {
        Self::essay_tools(this)
            .into_iter()
            .map(|t| t.name)
            .collect()
    }
    fn validate_essay(this: &Arc<Mutex<Self>>) -> Result<(), String>;

    fn input_schema_tools(this: &Arc<Mutex<Self>>) -> Vec<super::InventionTool>;
    fn input_schema_tool_names(this: &Arc<Mutex<Self>>) -> Vec<String> {
        Self::input_schema_tools(this)
            .into_iter()
            .map(|t| t.name)
            .collect()
    }
    fn validate_input_schema(this: &Arc<Mutex<Self>>) -> Result<(), String>;

    fn essay_tasks_tools(this: &Arc<Mutex<Self>>) -> Vec<super::InventionTool>;
    fn essay_tasks_tool_names(this: &Arc<Mutex<Self>>) -> Vec<String> {
        Self::essay_tasks_tools(this)
            .into_iter()
            .map(|t| t.name)
            .collect()
    }
    fn validate_essay_tasks(this: &Arc<Mutex<Self>>) -> Result<(), String>;

    fn tasks_tools(this: &Arc<Mutex<Self>>) -> Vec<super::InventionTool>;
    fn tasks_tool_names(this: &Arc<Mutex<Self>>) -> Vec<String> {
        Self::tasks_tools(this)
            .into_iter()
            .map(|t| t.name)
            .collect()
    }
    fn validate_function(this: &Arc<Mutex<Self>>) -> Result<(), String>;
    fn build_function(this: &Arc<Mutex<Self>>) -> Option<crate::functions::FullRemoteFunction>;

    fn description_tools(this: &Arc<Mutex<Self>>) -> Vec<super::InventionTool>;
    fn description_tool_names(this: &Arc<Mutex<Self>>) -> Vec<String> {
        Self::description_tools(this)
            .into_iter()
            .map(|t| t.name)
            .collect()
    }
    fn validate_description(this: &Arc<Mutex<Self>>) -> Result<(), String>;

    fn write_readme(this: &Arc<Mutex<Self>>);

    fn replace_placeholders(
        this: &Arc<Mutex<Self>>,
        paths: &[crate::RemotePath],
    );
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, arbitrary::Arbitrary)]
#[serde(tag = "type")]
#[schemars(rename = "functions.inventions.state.State")]
pub enum State {
    #[schemars(title = "AlphaScalarBranch")]
    #[serde(rename = "alpha.scalar.branch.function")]
    AlphaScalarBranch(AlphaScalarBranchState),
    #[schemars(title = "AlphaScalarLeaf")]
    #[serde(rename = "alpha.scalar.leaf.function")]
    AlphaScalarLeaf(AlphaScalarLeafState),
    #[schemars(title = "AlphaVectorBranch")]
    #[serde(rename = "alpha.vector.branch.function")]
    AlphaVectorBranch(AlphaVectorBranchState),
    #[schemars(title = "AlphaVectorLeaf")]
    #[serde(rename = "alpha.vector.leaf.function")]
    AlphaVectorLeaf(AlphaVectorLeafState),
}

impl State {
    /// Validates the initial state's input schema and tasks.
    ///
    /// Returns an error if the input schema is invalid or if the tasks
    /// are not valid for the provided input schema.
    pub fn validate_initial_state(
        &self,
        children: Option<&std::collections::HashMap<String, crate::functions::FullRemoteFunction>>,
    ) -> Result<(), String> {
        match self {
            State::AlphaScalarBranch(s) => s.validate_initial_state(children),
            State::AlphaScalarLeaf(s) => s.validate_initial_state(),
            State::AlphaVectorBranch(s) => s.validate_initial_state(children),
            State::AlphaVectorLeaf(s) => s.validate_initial_state(),
        }
    }

    /// Sets the checker seed on the inner state variant.
    pub fn set_checker_seed(&mut self, seed: Option<i64>) {
        match self {
            State::AlphaScalarBranch(s) => s.checker_seed = seed,
            State::AlphaScalarLeaf(s) => s.checker_seed = seed,
            State::AlphaVectorBranch(s) => s.checker_seed = seed,
            State::AlphaVectorLeaf(s) => s.checker_seed = seed,
        }
    }

    /// Returns the predicted tasks length, if set.
    pub fn tasks_length(&self) -> Option<u64> {
        match self {
            State::AlphaScalarBranch(s) => s.tasks_length,
            State::AlphaScalarLeaf(s) => s.tasks_length,
            State::AlphaVectorBranch(s) => s.tasks_length,
            State::AlphaVectorLeaf(s) => s.tasks_length,
        }
    }

    /// Returns a reference to the params.
    pub fn params(&self) -> &Params {
        match self {
            State::AlphaScalarBranch(s) => &s.params,
            State::AlphaScalarLeaf(s) => &s.params,
            State::AlphaVectorBranch(s) => &s.params,
            State::AlphaVectorLeaf(s) => &s.params,
        }
    }

    /// Returns the prompt type for this state variant.
    pub fn prompt_type(&self) -> super::prompts::StepPromptType {
        match self {
            State::AlphaScalarBranch(_) => super::prompts::StepPromptType::AlphaScalarBranchFunction,
            State::AlphaScalarLeaf(_) => super::prompts::StepPromptType::AlphaScalarLeafFunction,
            State::AlphaVectorBranch(_) => super::prompts::StepPromptType::AlphaVectorBranchFunction,
            State::AlphaVectorLeaf(_) => super::prompts::StepPromptType::AlphaVectorLeafFunction,
        }
    }

    /// Returns a reference to the params name.
    pub fn name(&self) -> &str {
        match self {
            State::AlphaScalarBranch(s) => &s.params.name,
            State::AlphaScalarLeaf(s) => &s.params.name,
            State::AlphaVectorBranch(s) => &s.params.name,
            State::AlphaVectorLeaf(s) => &s.params.name,
        }
    }

    /// Replaces placeholder tasks with real function tasks using the given paths.
    /// Matches by `repository == name`. No-op for leaf states.
    pub fn replace_placeholders(
        &mut self,
        paths: &[crate::RemotePath],
    ) {
        match self {
            State::AlphaScalarBranch(s) => s.replace_placeholders(paths),
            State::AlphaScalarLeaf(s) => s.replace_placeholders(paths),
            State::AlphaVectorBranch(s) => s.replace_placeholders(paths),
            State::AlphaVectorLeaf(s) => s.replace_placeholders(paths),
        }
    }

    /// Builds the `FullRemoteFunction` from the current state.
    /// Returns `None` if required fields are missing.
    pub fn build_function(&self) -> Option<crate::functions::FullRemoteFunction> {
        match self {
            State::AlphaScalarBranch(s) => s.build_function(),
            State::AlphaScalarLeaf(s) => s.build_function(),
            State::AlphaVectorBranch(s) => s.build_function(),
            State::AlphaVectorLeaf(s) => s.build_function(),
        }
    }

    /// Regenerates the README from the current state (description + sub-function URLs).
    pub fn write_readme(&mut self) {
        match self {
            State::AlphaScalarBranch(s) => s.write_readme(),
            State::AlphaScalarLeaf(s) => s.write_readme(),
            State::AlphaVectorBranch(s) => s.write_readme(),
            State::AlphaVectorLeaf(s) => s.write_readme(),
        }
    }

    /// Extracts child `ParamsState` values from placeholder tasks in branch states.
    /// Returns an empty vec for leaf states.
    pub fn placeholder_children(&self) -> Vec<ParamsState> {
        match self {
            State::AlphaScalarLeaf(_) | State::AlphaVectorLeaf(_) => vec![],
            State::AlphaScalarBranch(s) => {
                let tasks = match &s.tasks {
                    Some(tasks) => tasks,
                    None => return vec![],
                };
                tasks.iter().filter_map(|task| match task {
                    crate::functions::alpha_scalar::BranchTaskExpression::PlaceholderScalarFunction(t) => {
                        Some(ParamsState::AlphaScalar(AlphaScalarState {
                            params: t.params.clone(),
                            input_schema: Some(t.input_schema.clone()),
                        }))
                    }
                    _ => None,
                }).collect()
            }
            State::AlphaVectorBranch(s) => {
                let tasks = match &s.tasks {
                    Some(tasks) => tasks,
                    None => return vec![],
                };
                tasks.iter().filter_map(|task| match task {
                    crate::functions::alpha_vector::BranchTaskExpression::PlaceholderScalarFunction(t) => {
                        Some(ParamsState::AlphaScalar(AlphaScalarState {
                            params: t.params.clone(),
                            input_schema: Some(t.input_schema.clone()),
                        }))
                    }
                    crate::functions::alpha_vector::BranchTaskExpression::PlaceholderVectorFunction(t) => {
                        Some(ParamsState::AlphaVector(AlphaVectorState {
                            params: t.params.clone(),
                            input_schema: Some(t.input_schema.clone()),
                        }))
                    }
                    _ => None,
                }).collect()
            }
        }
    }

    pub fn serialize_into_files(&self) -> std::collections::HashMap<&'static str, String> {
        let files = match self {
            State::AlphaScalarBranch(s) => s.serialize_into_files(),
            State::AlphaScalarLeaf(s) => s.serialize_into_files(),
            State::AlphaVectorBranch(s) => s.serialize_into_files(),
            State::AlphaVectorLeaf(s) => s.serialize_into_files(),
        };
        files.into_hashmap()
    }

}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type")]
#[schemars(rename = "functions.inventions.state.ParamsState")]
pub enum ParamsState {
    #[schemars(title = "AlphaScalarBranch")]
    #[serde(rename = "alpha.scalar.branch.function")]
    AlphaScalarBranch(AlphaScalarBranchState),
    #[schemars(title = "AlphaScalarLeaf")]
    #[serde(rename = "alpha.scalar.leaf.function")]
    AlphaScalarLeaf(AlphaScalarLeafState),
    #[schemars(title = "AlphaVectorBranch")]
    #[serde(rename = "alpha.vector.branch.function")]
    AlphaVectorBranch(AlphaVectorBranchState),
    #[schemars(title = "AlphaVectorLeaf")]
    #[serde(rename = "alpha.vector.leaf.function")]
    AlphaVectorLeaf(AlphaVectorLeafState),
    #[schemars(title = "AlphaScalar")]
    #[serde(
        rename = "alpha.scalar.function",
        alias = "placeholder.alpha.scalar.function"
    )]
    AlphaScalar(AlphaScalarState),
    #[schemars(title = "AlphaVector")]
    #[serde(
        rename = "alpha.vector.function",
        alias = "placeholder.alpha.vector.function"
    )]
    AlphaVector(AlphaVectorState),
}

impl ParamsState {
    /// Returns the prompt type for this state variant.
    /// For unrouted `AlphaScalar`/`AlphaVector`, routes based on depth.
    pub fn prompt_type(&self) -> super::prompts::StepPromptType {
        match self {
            ParamsState::AlphaScalarBranch(_) => super::prompts::StepPromptType::AlphaScalarBranchFunction,
            ParamsState::AlphaScalarLeaf(_) => super::prompts::StepPromptType::AlphaScalarLeafFunction,
            ParamsState::AlphaVectorBranch(_) => super::prompts::StepPromptType::AlphaVectorBranchFunction,
            ParamsState::AlphaVectorLeaf(_) => super::prompts::StepPromptType::AlphaVectorLeafFunction,
            ParamsState::AlphaScalar(s) => {
                if s.params.depth == 0 {
                    super::prompts::StepPromptType::AlphaScalarLeafFunction
                } else {
                    super::prompts::StepPromptType::AlphaScalarBranchFunction
                }
            }
            ParamsState::AlphaVector(s) => {
                if s.params.depth == 0 {
                    super::prompts::StepPromptType::AlphaVectorLeafFunction
                } else {
                    super::prompts::StepPromptType::AlphaVectorBranchFunction
                }
            }
        }
    }

    pub fn route(self) -> State {
        match self {
            ParamsState::AlphaScalarBranch(s) => State::AlphaScalarBranch(s),
            ParamsState::AlphaScalarLeaf(s) => State::AlphaScalarLeaf(s),
            ParamsState::AlphaVectorBranch(s) => State::AlphaVectorBranch(s),
            ParamsState::AlphaVectorLeaf(s) => State::AlphaVectorLeaf(s),
            ParamsState::AlphaScalar(s) => {
                if s.params.depth == 0 {
                    State::AlphaScalarLeaf(AlphaScalarLeafState {
                        params: s.params,
                        essay: None,
                        input_schema: s.input_schema,
                        essay_tasks: None,
                        tasks: None,
                        tasks_length: None,
                        description: None,
                        readme: None,
                        checker_seed: None,
                    })
                } else {
                    State::AlphaScalarBranch(AlphaScalarBranchState {
                        params: s.params,
                        essay: None,
                        input_schema: s.input_schema,
                        essay_tasks: None,
                        tasks: None,
                        tasks_length: None,
                        description: None,
                        readme: None,
                        checker_seed: None,
                    })
                }
            }
            ParamsState::AlphaVector(s) => {
                if s.params.depth == 0 {
                    State::AlphaVectorLeaf(AlphaVectorLeafState {
                        params: s.params,
                        essay: None,
                        input_schema: s.input_schema,
                        essay_tasks: None,
                        tasks: None,
                        tasks_length: None,
                        description: None,
                        readme: None,
                        checker_seed: None,
                    })
                } else {
                    State::AlphaVectorBranch(AlphaVectorBranchState {
                        params: s.params,
                        essay: None,
                        input_schema: s.input_schema,
                        essay_tasks: None,
                        tasks: None,
                        tasks_length: None,
                        description: None,
                        readme: None,
                        checker_seed: None,
                    })
                }
            }
        }
    }

    pub const fn filenames() -> &'static [&'static str] {
        files::Files::filenames()
    }

    pub fn serialize_into_files(&self) -> std::collections::HashMap<&'static str, String> {
        let files = match self {
            ParamsState::AlphaScalarBranch(s) => s.serialize_into_files(),
            ParamsState::AlphaScalarLeaf(s) => s.serialize_into_files(),
            ParamsState::AlphaVectorBranch(s) => s.serialize_into_files(),
            ParamsState::AlphaVectorLeaf(s) => s.serialize_into_files(),
            ParamsState::AlphaScalar(s) => s.serialize_into_files(),
            ParamsState::AlphaVector(s) => s.serialize_into_files(),
        };
        files.into_hashmap()
    }

    pub fn deserialize_from_files(map: std::collections::HashMap<&'static str, String>) -> Result<Option<Self>, error::Error> {
        let files = files::Files::from_hashmap(map)?;

        // Deserialize function.json if present to determine the routed variant
        let function: Option<crate::functions::FullRemoteFunction> = files.function_json.as_ref()
            .map(|json| {
                let mut de = serde_json::Deserializer::from_str(json);
                serde_path_to_error::deserialize(&mut de)
                    .map_err(|e| error::Error::Deserialize {
                        file: files::Files::FUNCTION_JSON,
                        source: e,
                    })
            })
            .transpose()?;

        match function {
            Some(crate::functions::FullRemoteFunction::Alpha(
                crate::functions::AlphaRemoteFunction::Scalar(
                    scalar @ crate::functions::alpha_scalar::RemoteFunction::Leaf { .. }
                )
            )) => Ok(Some(ParamsState::AlphaScalarLeaf(AlphaScalarLeafState::deserialize_from_files(Some(scalar), &files)?))),
            Some(crate::functions::FullRemoteFunction::Alpha(
                crate::functions::AlphaRemoteFunction::Scalar(
                    scalar @ crate::functions::alpha_scalar::RemoteFunction::Branch { .. }
                )
            )) => Ok(Some(ParamsState::AlphaScalarBranch(AlphaScalarBranchState::deserialize_from_files(Some(scalar), &files)?))),
            Some(crate::functions::FullRemoteFunction::Alpha(
                crate::functions::AlphaRemoteFunction::Vector(
                    vector @ crate::functions::alpha_vector::RemoteFunction::Leaf { .. }
                )
            )) => Ok(Some(ParamsState::AlphaVectorLeaf(AlphaVectorLeafState::deserialize_from_files(Some(vector), &files)?))),
            Some(crate::functions::FullRemoteFunction::Alpha(
                crate::functions::AlphaRemoteFunction::Vector(
                    vector @ crate::functions::alpha_vector::RemoteFunction::Branch { .. }
                )
            )) => Ok(Some(ParamsState::AlphaVectorBranch(AlphaVectorBranchState::deserialize_from_files(Some(vector), &files)?))),
            Some(other) => Err(error::Error::UnrecognizedFunctionType(format!("{:?}", std::mem::discriminant(&other)))),
            // No function.json — composite AlphaScalar/AlphaVector from params + optional input_schema
            None => {
                let input_schema: Option<InputSchema> = files.input_schema_json.as_ref()
                    .map(|json| {
                        let mut de = serde_json::Deserializer::from_str(json);
                        serde_path_to_error::deserialize(&mut de)
                            .map_err(|e| error::Error::Deserialize {
                                file: files::Files::INPUT_SCHEMA_JSON,
                                source: e,
                            })
                    })
                    .transpose()?;

                match input_schema {
                    Some(InputSchema::Scalar { schema }) => {
                        Ok(Some(ParamsState::AlphaScalar(AlphaScalarState::deserialize_from_files(Some(schema), &files)?)))
                    }
                    Some(InputSchema::Vector { schema }) => {
                        Ok(Some(ParamsState::AlphaVector(AlphaVectorState::deserialize_from_files(Some(schema), &files)?)))
                    }
                    None => Ok(None),
                }
            }
        }
    }
}
