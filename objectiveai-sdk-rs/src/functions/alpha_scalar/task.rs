use crate::{agent, functions};
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
#[schemars(rename = "functions.alpha_scalar.BranchTaskExpression")]
pub enum BranchTaskExpression {
    #[schemars(title = "ScalarFunction")]
    #[serde(rename = "alpha.scalar.function")]
    ScalarFunction(ScalarFunctionTaskExpression),
    #[schemars(title = "PlaceholderScalarFunction")]
    #[serde(rename = "placeholder.alpha.scalar.function")]
    PlaceholderScalarFunction(PlaceholderScalarFunctionTaskExpression),
}

impl BranchTaskExpression {
    pub fn url(&self) -> Option<String> {
        match self {
            BranchTaskExpression::ScalarFunction(task) => Some(task.url()),
            BranchTaskExpression::PlaceholderScalarFunction(_) => None,
        }
    }

    pub fn transpile(self) -> functions::TaskExpression {
        match self {
            BranchTaskExpression::ScalarFunction(task) => {
                functions::TaskExpression::ScalarFunction(task.transpile())
            }
            BranchTaskExpression::PlaceholderScalarFunction(task) => {
                functions::TaskExpression::PlaceholderScalarFunction(
                    task.transpile(),
                )
            }
        }
    }

    pub fn is_placeholder(&self) -> bool {
        match self {
            BranchTaskExpression::ScalarFunction(_) => false,
            BranchTaskExpression::PlaceholderScalarFunction(_) => true,
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
#[schemars(rename = "functions.alpha_scalar.LeafTaskExpression")]
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
#[schemars(rename = "functions.alpha_scalar.ScalarFunctionTaskExpression")]
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
            map: None,
            input:
                super::expression::scalar_function_input_value_expression::transpile(
                    self.input,
                ),
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
    rename = "functions.alpha_scalar.PlaceholderScalarFunctionTaskExpression"
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
            map: None,
            input:
                super::expression::scalar_function_input_value_expression::transpile(
                    self.input,
                ),
            output: functions::expression::Expression::Special(
                functions::expression::Special::Output,
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
#[schemars(rename = "functions.alpha_scalar.VectorCompletionTaskExpression")]
pub struct VectorCompletionTaskExpression {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub skip: Option<functions::expression::Expression>,
    pub messages: functions::expression::Expression,
    pub responses: Vec<agent::completions::message::RichContent>,
}

impl VectorCompletionTaskExpression {
    pub fn transpile(self) -> functions::VectorCompletionTaskExpression {
        functions::VectorCompletionTaskExpression {
            skip: self.skip,
            map: None,
            messages: functions::expression::WithExpression::Expression(
                self.messages,
            ),
            responses: functions::expression::WithExpression::Value(
                self.responses
                    .into_iter()
                    .map(agent::completions::message::RichContentExpression::from)
                    .map(functions::expression::WithExpression::Value)
                    .collect(),
            ),
            output: functions::expression::Expression::Special(
                functions::expression::Special::TaskOutputWeightedSum,
            ),
        }
    }
}
