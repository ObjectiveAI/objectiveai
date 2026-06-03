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
#[schemars(rename = "functions.alpha_scalar.RemoteFunction")]
pub enum RemoteFunction {
    #[schemars(title = "Branch")]
    #[serde(rename = "alpha.scalar.branch.function")]
    Branch {
        description: String,
        input_schema: super::expression::ScalarFunctionInputSchema,
        tasks: Vec<super::BranchTaskExpression>,
    },
    #[schemars(title = "Leaf")]
    #[serde(rename = "alpha.scalar.leaf.function")]
    Leaf {
        description: String,
        input_schema: super::expression::ScalarFunctionInputSchema,
        tasks: Vec<super::LeafTaskExpression>,
    },
}

impl RemoteFunction {
    pub fn tasks(&self) -> &[super::BranchTaskExpression] {
        match self {
            RemoteFunction::Branch { tasks, .. } => tasks,
            RemoteFunction::Leaf { .. } => &[],
        }
    }

    pub fn remotes(&self) -> impl Iterator<Item = &crate::RemotePath> {
        self.tasks().iter().filter_map(|task| match task {
            super::BranchTaskExpression::ScalarFunction(t) => Some(&t.path),
            _ => None,
        })
    }

    pub fn transpile(self) -> functions::RemoteFunction {
        match self {
            RemoteFunction::Branch {
                description,
                input_schema,
                tasks,
            } => functions::RemoteFunction::Scalar {
                description,
                input_schema:
                    super::expression::scalar_function_input_schema::transpile(
                        input_schema,
                    ),
                tasks: tasks
                    .into_iter()
                    .map(super::BranchTaskExpression::transpile)
                    .collect(),
            },
            RemoteFunction::Leaf {
                description,
                input_schema,
                tasks,
            } => functions::RemoteFunction::Scalar {
                description,
                input_schema:
                    super::expression::scalar_function_input_schema::transpile(
                        input_schema,
                    ),
                tasks: tasks
                    .into_iter()
                    .map(super::LeafTaskExpression::transpile)
                    .collect(),
            },
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
#[schemars(rename = "functions.alpha_scalar.InlineFunction")]
pub enum InlineFunction {
    #[schemars(title = "Branch")]
    #[serde(rename = "alpha.scalar.branch.function")]
    Branch {
        tasks: Vec<super::BranchTaskExpression>,
    },
    #[schemars(title = "Leaf")]
    #[serde(rename = "alpha.scalar.leaf.function")]
    Leaf {
        tasks: Vec<super::LeafTaskExpression>,
    },
}

impl InlineFunction {
    pub fn tasks(&self) -> &[super::BranchTaskExpression] {
        match self {
            InlineFunction::Branch { tasks, .. } => tasks,
            InlineFunction::Leaf { .. } => &[],
        }
    }

    pub fn remotes(&self) -> impl Iterator<Item = &crate::RemotePath> {
        self.tasks().iter().filter_map(|task| match task {
            super::BranchTaskExpression::ScalarFunction(t) => Some(&t.path),
            _ => None,
        })
    }

    pub fn transpile(self) -> functions::InlineFunction {
        match self {
            InlineFunction::Branch { tasks } => {
                functions::InlineFunction::Scalar {
                    tasks: tasks
                        .into_iter()
                        .map(super::BranchTaskExpression::transpile)
                        .collect(),
                }
            }
            InlineFunction::Leaf { tasks } => {
                functions::InlineFunction::Scalar {
                    tasks: tasks
                        .into_iter()
                        .map(super::LeafTaskExpression::transpile)
                        .collect(),
                }
            }
        }
    }
}
