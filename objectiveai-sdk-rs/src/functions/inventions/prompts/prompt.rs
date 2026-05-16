use crate::functions;
use crate::functions::expression::WithExpression;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The type of invention state a prompt applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, arbitrary::Arbitrary)]
#[schemars(rename = "functions.inventions.prompts.StepPromptType")]
pub enum StepPromptType {
    #[serde(rename = "alpha.scalar.branch.function")]
    AlphaScalarBranchFunction,
    #[serde(rename = "alpha.scalar.leaf.function")]
    AlphaScalarLeafFunction,
    #[serde(rename = "alpha.vector.branch.function")]
    AlphaVectorBranchFunction,
    #[serde(rename = "alpha.vector.leaf.function")]
    AlphaVectorLeafFunction,
}

/// A prompt for a single invention step, applicable to one or more state types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, arbitrary::Arbitrary)]
#[schemars(rename = "functions.inventions.prompts.StepPromptExpression")]
pub struct StepPromptExpression {
    pub r#type: Vec<StepPromptType>,
    pub value: WithExpression<String>,
}

impl StepPromptExpression {
    pub fn compile(
        self,
        params: &functions::expression::Params,
    ) -> Result<String, functions::expression::ExpressionError> {
        self.value.compile_one(params)
    }
}

/// A prompt specification that is either an inline prompt definition
/// or a remote path reference.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "functions.inventions.prompts.InlinePromptOrRemoteCommitOptional")]
pub enum InlinePromptOrRemoteCommitOptional {
    #[schemars(title = "Inline")]
    Inline(InlinePrompt),
    #[schemars(title = "Remote")]
    Remote(crate::RemotePathCommitOptional),
}

/// A Prompt definition, either remote or inline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "functions.inventions.prompts.Prompt")]
pub enum Prompt {
    /// A remote prompt with metadata.
    #[schemars(title = "Remote")]
    Remote(RemotePrompt),
    /// An inline prompt definition.
    #[schemars(title = "Inline")]
    Inline(InlinePrompt),
}

impl Prompt {
    pub fn essay(&self) -> &[StepPromptExpression] {
        match self {
            Prompt::Remote(r) => r.essay(),
            Prompt::Inline(i) => i.essay(),
        }
    }
    pub fn input_schema(&self) -> &[StepPromptExpression] {
        match self {
            Prompt::Remote(r) => r.input_schema(),
            Prompt::Inline(i) => i.input_schema(),
        }
    }
    pub fn essay_tasks(&self) -> &[StepPromptExpression] {
        match self {
            Prompt::Remote(r) => r.essay_tasks(),
            Prompt::Inline(i) => i.essay_tasks(),
        }
    }
    pub fn tasks(&self) -> &[StepPromptExpression] {
        match self {
            Prompt::Remote(r) => r.tasks(),
            Prompt::Inline(i) => i.tasks(),
        }
    }
    pub fn description(&self) -> &[StepPromptExpression] {
        match self {
            Prompt::Remote(r) => r.description(),
            Prompt::Inline(i) => i.description(),
        }
    }
    pub fn supports_type(&self, t: StepPromptType) -> bool {
        match self {
            Prompt::Remote(r) => r.supports_type(t),
            Prompt::Inline(i) => i.supports_type(t),
        }
    }
    pub fn essay_for_type(&self, t: StepPromptType) -> Option<&StepPromptExpression> {
        match self {
            Prompt::Remote(r) => r.essay_for_type(t),
            Prompt::Inline(i) => i.essay_for_type(t),
        }
    }
    pub fn input_schema_for_type(&self, t: StepPromptType) -> Option<&StepPromptExpression> {
        match self {
            Prompt::Remote(r) => r.input_schema_for_type(t),
            Prompt::Inline(i) => i.input_schema_for_type(t),
        }
    }
    pub fn essay_tasks_for_type(&self, t: StepPromptType) -> Option<&StepPromptExpression> {
        match self {
            Prompt::Remote(r) => r.essay_tasks_for_type(t),
            Prompt::Inline(i) => i.essay_tasks_for_type(t),
        }
    }
    pub fn tasks_for_type(&self, t: StepPromptType) -> Option<&StepPromptExpression> {
        match self {
            Prompt::Remote(r) => r.tasks_for_type(t),
            Prompt::Inline(i) => i.tasks_for_type(t),
        }
    }
    pub fn description_for_type(&self, t: StepPromptType) -> Option<&StepPromptExpression> {
        match self {
            Prompt::Remote(r) => r.description_for_type(t),
            Prompt::Inline(i) => i.description_for_type(t),
        }
    }
}

/// A remote prompt with description and inline prompt fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.inventions.prompts.RemotePrompt")]
pub struct RemotePrompt {
    /// Human-readable description of the prompt.
    pub description: String,
    #[serde(flatten)]
    #[schemars(schema_with = "crate::flatten_schema::<InlinePrompt>")]
    pub inner: InlinePrompt,
}

impl RemotePrompt {
    pub fn essay(&self) -> &[StepPromptExpression] { self.inner.essay() }
    pub fn input_schema(&self) -> &[StepPromptExpression] { self.inner.input_schema() }
    pub fn essay_tasks(&self) -> &[StepPromptExpression] { self.inner.essay_tasks() }
    pub fn tasks(&self) -> &[StepPromptExpression] { self.inner.tasks() }
    pub fn description(&self) -> &[StepPromptExpression] { self.inner.description() }
    pub fn supports_type(&self, t: StepPromptType) -> bool { self.inner.supports_type(t) }
    pub fn essay_for_type(&self, t: StepPromptType) -> Option<&StepPromptExpression> { self.inner.essay_for_type(t) }
    pub fn input_schema_for_type(&self, t: StepPromptType) -> Option<&StepPromptExpression> { self.inner.input_schema_for_type(t) }
    pub fn essay_tasks_for_type(&self, t: StepPromptType) -> Option<&StepPromptExpression> { self.inner.essay_tasks_for_type(t) }
    pub fn tasks_for_type(&self, t: StepPromptType) -> Option<&StepPromptExpression> { self.inner.tasks_for_type(t) }
    pub fn description_for_type(&self, t: StepPromptType) -> Option<&StepPromptExpression> { self.inner.description_for_type(t) }
}

/// Inline invention prompt configuration for all steps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, arbitrary::Arbitrary)]
#[schemars(rename = "functions.inventions.prompts.InlinePrompt")]
pub struct InlinePrompt {
    pub essay_step: Vec<StepPromptExpression>,
    pub input_schema_step: Vec<StepPromptExpression>,
    pub essay_tasks_step: Vec<StepPromptExpression>,
    pub tasks_step: Vec<StepPromptExpression>,
    pub description_step: Vec<StepPromptExpression>,
}

fn find_for_type(steps: &[StepPromptExpression], t: StepPromptType) -> Option<&StepPromptExpression> {
    steps.iter().find(|s| s.r#type.contains(&t))
}

fn has_type(steps: &[StepPromptExpression], t: StepPromptType) -> bool {
    find_for_type(steps, t).is_some()
}

impl InlinePrompt {
    pub fn essay(&self) -> &[StepPromptExpression] { &self.essay_step }
    pub fn input_schema(&self) -> &[StepPromptExpression] { &self.input_schema_step }
    pub fn essay_tasks(&self) -> &[StepPromptExpression] { &self.essay_tasks_step }
    pub fn tasks(&self) -> &[StepPromptExpression] { &self.tasks_step }
    pub fn description(&self) -> &[StepPromptExpression] { &self.description_step }

    pub fn supports_type(&self, t: StepPromptType) -> bool {
        has_type(&self.essay_step, t)
            && has_type(&self.input_schema_step, t)
            && has_type(&self.essay_tasks_step, t)
            && has_type(&self.tasks_step, t)
            && has_type(&self.description_step, t)
    }

    pub fn essay_for_type(&self, t: StepPromptType) -> Option<&StepPromptExpression> { find_for_type(&self.essay_step, t) }
    pub fn input_schema_for_type(&self, t: StepPromptType) -> Option<&StepPromptExpression> { find_for_type(&self.input_schema_step, t) }
    pub fn essay_tasks_for_type(&self, t: StepPromptType) -> Option<&StepPromptExpression> { find_for_type(&self.essay_tasks_step, t) }
    pub fn tasks_for_type(&self, t: StepPromptType) -> Option<&StepPromptExpression> { find_for_type(&self.tasks_step, t) }
    pub fn description_for_type(&self, t: StepPromptType) -> Option<&StepPromptExpression> { find_for_type(&self.description_step, t) }
}
