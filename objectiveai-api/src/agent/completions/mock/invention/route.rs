/// Discovered invention step from the prompt text, available tool names,
/// and optionally from calling the invention tools themselves.
///
/// The mock client inspects the invention tools to determine which step of
/// the invention pipeline it is being asked to execute, and which of the 4
/// routes (scalar leaf, scalar branch, vector leaf, vector branch) applies.
///
/// Discovery uses three layers:
/// 1. **Tool names** — which invention tools are present (e.g. `WriteEssay`,
///    `AppendTask`) determines the step.
/// 2. **Schema tool names** — `Read*JsonSchema` tools differ by route at the
///    InputSchema and Tasks steps.
/// 3. **Tool invocation** — calling read tools (e.g. `ReadInputSchema`,
///    `ReadTask`) and inspecting their output reveals the route at steps
///    where tool names alone are ambiguous (EssayTasks, Description).
///
/// See `DISCOVERY.md` for the full discovery logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventionStep {
    // Step 1: Essay
    // Prompt differs ("Scalar Function" vs "Vector Function") but tools are identical.
    EssayScalar,
    EssayVector,

    // Step 2: Input Schema
    // Vector has Readfunctions.alpha_vector.expression.VectorFunctionInputSchemaSchema;
    // scalar does not.
    InputSchemaScalar,
    InputSchemaVector,

    // Step 3: Essay Tasks
    // Tool names and prompt are identical across all 4 routes.
    // Scalar vs vector: call ReadInputSchema → ScalarFunctionInputSchema
    // has `"type": "object"` at root, VectorFunctionInputSchema has
    // `"input_split"` / `"input_merge"` fields.
    // Leaf vs branch: unknown at this step (tasks not yet written).
    EssayTasksScalar,
    EssayTasksVector,

    // Step 4: Tasks (Body)
    // Fully distinguishable via schema tool names.
    TasksScalarLeaf,
    TasksScalarBranch,
    TasksVectorLeaf,
    TasksVectorBranch,

    // Step 5: Description
    // Tool names and prompt are identical across all 4 routes.
    // Scalar vs vector: call ReadInputSchema (same as EssayTasks).
    // Leaf vs branch: call ReadTask(0) → leaf tasks serialize as
    // VectorCompletion expressions, branch tasks as Placeholder expressions.
    DescriptionScalarLeaf,
    DescriptionScalarBranch,
    DescriptionVectorLeaf,
    DescriptionVectorBranch,
}

impl InventionStep {
    /// Discover which invention step we're in from tool names and prompt.
    ///
    /// This performs layer-1 (tool names) and layer-2 (schema tool names)
    /// discovery. For steps that require layer-3 (tool invocation), this
    /// returns a partially-resolved step that the caller must refine by
    /// calling invention tools.
    ///
    /// `prompt` is the user-facing prompt text (from messages or continuation).
    pub fn discover(tool_names: &[String], prompt: &str) -> Option<Self> {
        // Tool names arrive proxy-prefixed with the InventionServer's
        // server name (`oaifi`), so every literal below matches against
        // the full prefixed string.
        let has = |name: &str| tool_names.iter().any(|t| t == name);

        // Step 1: Essay — WriteEssay present, WriteInputSchema absent
        if has("oaifi_WriteEssay")
            && !has("oaifi_WriteInputSchema")
        {
            return Some(if prompt.contains("Scalar Function") {
                Self::EssayScalar
            } else {
                Self::EssayVector
            });
        }

        // Step 2: InputSchema — WriteInputSchema present
        if has("oaifi_WriteInputSchema") {
            return Some(
                if has("oaifi_Readfunctions_alpha_vector_expression_VectorFunctionInputSchemaSchema") {
                    Self::InputSchemaVector
                } else {
                    Self::InputSchemaScalar
                },
            );
        }

        // Step 3: EssayTasks — WriteEssayTasks present, AppendTask absent
        if has("oaifi_WriteEssayTasks")
            && !has("oaifi_AppendTask")
        {
            // Scalar vs vector requires calling ReadInputSchema (layer 3).
            // Return scalar as default — caller refines via `refine_with_input_schema`.
            return Some(Self::EssayTasksScalar);
        }

        // Step 4: Tasks — AppendTask present
        if has("oaifi_AppendTask") {
            return Some(if has("oaifi_Readagent_completions_message_MessageExpressionSchema") {
                // Leaf
                if tool_names.iter().any(|t| t.contains("alpha_scalar")) {
                    Self::TasksScalarLeaf
                } else {
                    Self::TasksVectorLeaf
                }
            } else if has("oaifi_Readfunctions_expression_InputValueSchema") {
                // Branch
                if tool_names.iter().any(|t| t.contains("alpha_scalar_Placeholder")) {
                    Self::TasksScalarBranch
                } else {
                    Self::TasksVectorBranch
                }
            } else {
                panic!("AppendTask present but neither leaf nor branch schema tools found. Tool names: {:?}", tool_names);
            });
        }

        // Step 5: Description — WriteDescription present
        if has("oaifi_WriteDescription") {
            // Requires layer-3 discovery. Return scalar leaf as default.
            return Some(Self::DescriptionScalarLeaf);
        }

        None
    }

    /// Refine an EssayTasks or Description step using the input schema JSON
    /// returned by calling `ReadInputSchema`.
    ///
    /// If the schema has an `items` field, it's a vector function.
    pub fn refine_with_input_schema(self, input_schema_json: &str) -> Self {
        let is_vector = serde_json::from_str::<serde_json::Value>(input_schema_json)
            .ok()
            .and_then(|v| v.get("items").cloned())
            .is_some();

        match self {
            Self::EssayTasksScalar if is_vector => Self::EssayTasksVector,
            Self::DescriptionScalarLeaf if is_vector => Self::DescriptionVectorLeaf,
            Self::DescriptionScalarBranch if is_vector => Self::DescriptionVectorBranch,
            other => other,
        }
    }

    /// Refine a Description step using the task JSON returned by calling
    /// `ReadTask(0)`.
    ///
    /// If the task contains `"placeholder"` in its keys, it's a branch.
    pub fn refine_with_task(self, task_json: &str) -> Self {
        let is_branch = task_json.contains("placeholder");

        match self {
            Self::DescriptionScalarLeaf if is_branch => Self::DescriptionScalarBranch,
            Self::DescriptionVectorLeaf if is_branch => Self::DescriptionVectorBranch,
            other => other,
        }
    }

    /// Whether this step needs the input schema for generating tool calls
    /// (tasks and essay_tasks steps).
    pub fn needs_input_schema(self) -> bool {
        matches!(
            self,
            Self::EssayTasksScalar
                | Self::EssayTasksVector
                | Self::TasksScalarLeaf
                | Self::TasksScalarBranch
                | Self::TasksVectorLeaf
                | Self::TasksVectorBranch
                | Self::DescriptionScalarLeaf
                | Self::DescriptionScalarBranch
                | Self::DescriptionVectorLeaf
                | Self::DescriptionVectorBranch
        )
    }
}
