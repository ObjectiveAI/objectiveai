use crate::functions;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(tag = "type")]
#[schemars(rename = "functions.alpha_vector.BranchTaskExpression")]
pub enum BranchTaskExpression {
    #[schemars(title = "ScalarFunction")]
    #[serde(rename = "alpha.scalar.function")]
    ScalarFunction(ScalarFunctionTaskExpression),
    #[schemars(title = "VectorFunction")]
    #[serde(rename = "alpha.vector.function")]
    VectorFunction(VectorFunctionTaskExpression),
    #[schemars(title = "PlaceholderScalarFunction")]
    #[serde(rename = "placeholder.alpha.scalar.function")]
    PlaceholderScalarFunction(PlaceholderScalarFunctionTaskExpression),
    #[schemars(title = "PlaceholderVectorFunction")]
    #[serde(rename = "placeholder.alpha.vector.function")]
    PlaceholderVectorFunction(PlaceholderVectorFunctionTaskExpression),
}

impl BranchTaskExpression {
    pub fn url(&self) -> Option<String> {
        match self {
            BranchTaskExpression::ScalarFunction(task) => Some(task.url()),
            BranchTaskExpression::VectorFunction(task) => Some(task.url()),
            BranchTaskExpression::PlaceholderScalarFunction(_) => None,
            BranchTaskExpression::PlaceholderVectorFunction(_) => None,
        }
    }

    pub fn transpile(self) -> functions::TaskExpression {
        match self {
            BranchTaskExpression::ScalarFunction(task) => {
                functions::TaskExpression::ScalarFunction(task.transpile())
            }
            BranchTaskExpression::VectorFunction(task) => {
                functions::TaskExpression::VectorFunction(task.transpile())
            }
            BranchTaskExpression::PlaceholderScalarFunction(task) => {
                functions::TaskExpression::PlaceholderScalarFunction(
                    task.transpile(),
                )
            }
            BranchTaskExpression::PlaceholderVectorFunction(task) => {
                functions::TaskExpression::PlaceholderVectorFunction(
                    task.transpile(),
                )
            }
        }
    }

    pub fn is_placeholder(&self) -> bool {
        match self {
            BranchTaskExpression::ScalarFunction(_) => false,
            BranchTaskExpression::VectorFunction(_) => false,
            BranchTaskExpression::PlaceholderScalarFunction(_) => true,
            BranchTaskExpression::PlaceholderVectorFunction(_) => true,
        }
    }
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(tag = "type")]
#[schemars(rename = "functions.alpha_vector.LeafTaskExpression")]
pub enum LeafTaskExpression {
    #[serde(rename = "vector.completion")]
    VectorCompletion(VectorCompletionTaskExpression),
}

impl LeafTaskExpression {
    pub fn transpile(self) -> functions::TaskExpression {
        match self {
            LeafTaskExpression::VectorCompletion(task) => {
                functions::TaskExpression::VectorCompletion(task.transpile())
            }
        }
    }
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "functions.alpha_vector.ScalarFunctionTaskExpression")]
pub struct ScalarFunctionTaskExpression {
    #[serde(flatten)]
    #[schemars(schema_with = "crate::flatten_schema::<crate::RemotePath>")]
    pub path: crate::RemotePath,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub skip: Option<functions::expression::Expression>,
    pub input: super::expression::ScalarFunctionInputValueExpression,
}

impl ScalarFunctionTaskExpression {
    pub fn url(&self) -> String {
        self.path.url()
    }

    pub fn transpile(self) -> functions::ScalarFunctionTaskExpression {
        functions::ScalarFunctionTaskExpression {
            path: self.path,
            skip: self.skip,
            // Scalar sub-tasks of an alpha-vector parent must be mapped
            // over `input['items']`, otherwise expressions like
            // `input['items'][map]` see `map = None` at runtime. Mirrors
            // PlaceholderScalarFunctionTaskExpression::transpile below.
            map: Some(functions::expression::Expression::Special(
                functions::expression::Special::InputItemsOutputLength,
            )),
            input:
                super::expression::scalar_function_input_value_expression::transpile(
                    self.input,
                ),
            output: functions::expression::Expression::Special(
                functions::expression::Special::TaskOutputL1Normalized,
            ),
        }
    }
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "functions.alpha_vector.VectorFunctionTaskExpression")]
pub struct VectorFunctionTaskExpression {
    #[serde(flatten)]
    #[schemars(schema_with = "crate::flatten_schema::<crate::RemotePath>")]
    pub path: crate::RemotePath,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub skip: Option<functions::expression::Expression>,
    pub input: super::expression::VectorFunctionInputValueExpression,
}

impl VectorFunctionTaskExpression {
    pub fn url(&self) -> String {
        self.path.url()
    }

    pub fn transpile(self) -> functions::VectorFunctionTaskExpression {
        functions::VectorFunctionTaskExpression {
            path: self.path,
            skip: self.skip,
            map: None,
            input: self.input.transpile(),
            output: functions::expression::Expression::Special(
                functions::expression::Special::Output,
            ),
        }
    }
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(
    rename = "functions.alpha_vector.PlaceholderScalarFunctionTaskExpression"
)]
pub struct PlaceholderScalarFunctionTaskExpression {
    pub input_schema: super::expression::ScalarFunctionInputSchema,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub skip: Option<functions::expression::Expression>,
    pub input: super::expression::ScalarFunctionInputValueExpression,
}

impl PlaceholderScalarFunctionTaskExpression {
    pub fn transpile(
        self,
    ) -> functions::PlaceholderScalarFunctionTaskExpression {
        functions::PlaceholderScalarFunctionTaskExpression {
            input_schema:
                super::expression::scalar_function_input_schema::transpile(
                    self.input_schema,
                ),
            skip: self.skip,
            map: Some(functions::expression::Expression::Special(
                functions::expression::Special::InputItemsOutputLength,
            )),
            input:
                super::expression::scalar_function_input_value_expression::transpile(
                    self.input,
                ),
            output: functions::expression::Expression::Special(
                functions::expression::Special::TaskOutputL1Normalized,
            ),
        }
    }

    pub fn replace(
        self,
        path: &crate::RemotePath,
    ) -> ScalarFunctionTaskExpression {
        ScalarFunctionTaskExpression {
            path: path.clone(),
            skip: self.skip,
            input: self.input,
        }
    }
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(
    rename = "functions.alpha_vector.PlaceholderVectorFunctionTaskExpression"
)]
pub struct PlaceholderVectorFunctionTaskExpression {
    pub input_schema: super::expression::VectorFunctionInputSchema,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub skip: Option<functions::expression::Expression>,
    pub input: super::expression::VectorFunctionInputValueExpression,
}

impl PlaceholderVectorFunctionTaskExpression {
    pub fn transpile(
        self,
    ) -> functions::PlaceholderVectorFunctionTaskExpression {
        functions::PlaceholderVectorFunctionTaskExpression {
            input_schema: self.input_schema.transpile(),
            output_length: functions::expression::Expression::Special(
                functions::expression::Special::InputItemsOutputLength,
            ),
            input_split: functions::expression::Expression::Special(
                functions::expression::Special::InputItemsOptionalContextSplit,
            ),
            input_merge: functions::expression::Expression::Special(
                functions::expression::Special::InputItemsOptionalContextMerge,
            ),
            skip: self.skip,
            map: None,
            input: self.input.transpile(),
            output: functions::expression::Expression::Special(
                functions::expression::Special::Output,
            ),
        }
    }

    pub fn replace(
        self,
        path: &crate::RemotePath,
    ) -> VectorFunctionTaskExpression {
        VectorFunctionTaskExpression {
            path: path.clone(),
            skip: self.skip,
            input: self.input,
        }
    }
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "functions.alpha_vector.VectorCompletionTaskExpression")]
pub struct VectorCompletionTaskExpression {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub skip: Option<functions::expression::Expression>,
    pub messages: functions::expression::Expression,
    pub responses: functions::expression::Expression,
}

impl VectorCompletionTaskExpression {
    pub fn transpile(self) -> functions::VectorCompletionTaskExpression {
        functions::VectorCompletionTaskExpression {
            skip: self.skip,
            map: None,
            messages: functions::expression::WithExpression::Expression(
                self.messages,
            ),
            responses: functions::expression::WithExpression::Expression(
                self.responses,
            ),
            output: functions::expression::Expression::Special(
                functions::expression::Special::Output,
            ),
        }
    }
}
