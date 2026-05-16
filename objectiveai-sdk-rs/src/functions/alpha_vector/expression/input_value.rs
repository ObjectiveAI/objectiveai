use crate::functions;
use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub type ScalarFunctionInputValueExpression = functions::expression::Expression;

pub mod scalar_function_input_value_expression {
    use crate::functions;
    pub fn transpile(
        this: super::ScalarFunctionInputValueExpression,
    ) -> functions::expression::WithExpression<
        functions::expression::InputValueExpression,
    > {
        functions::expression::WithExpression::Expression(this)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, arbitrary::Arbitrary)]
#[schemars(rename = "functions.alpha_vector.expression.VectorFunctionInputValueExpression")]
pub struct VectorFunctionInputValueExpression {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub context: Option<functions::expression::Expression>,
    pub items: functions::expression::Expression,
}

impl VectorFunctionInputValueExpression {
    pub fn transpile(
        self,
    ) -> functions::expression::WithExpression<
        functions::expression::InputValueExpression,
    > {
        functions::expression::WithExpression::Value(
            functions::expression::InputValueExpression::Object({
                let mut map =
                    IndexMap::with_capacity(if self.context.is_some() {
                        2
                    } else {
                        1
                    });
                if let Some(context) = self.context {
                    map.insert(
                        "context".to_string(),
                        functions::expression::WithExpression::Expression(
                            context,
                        ),
                    );
                }
                map.insert(
                    "items".to_string(),
                    functions::expression::WithExpression::Expression(
                        self.items,
                    ),
                );
                map
            }),
        )
    }
}

pub type ScalarFunctionInputValue = IndexMap<String, functions::expression::InputValue>;

pub mod scalar_function_input_value {
    use crate::functions;
    pub fn transpile(
        this: super::ScalarFunctionInputValue,
    ) -> functions::expression::InputValue {
        functions::expression::InputValue::Object(this)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
#[schemars(rename = "functions.alpha_vector.expression.VectorFunctionInputValue")]
pub struct VectorFunctionInputValue {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub context: Option<IndexMap<String, functions::expression::InputValue>>,
    pub items: Vec<functions::expression::InputValue>,
}

impl VectorFunctionInputValue {
    pub fn transpile(self) -> functions::expression::InputValue {
        functions::expression::InputValue::Object({
            let mut map = IndexMap::with_capacity(if self.context.is_some() {
                2
            } else {
                1
            });
            if let Some(context) = self.context {
                map.insert(
                    "context".to_string(),
                    functions::expression::InputValue::Object(context),
                );
            }
            map.insert(
                "items".to_string(),
                functions::expression::InputValue::Array(self.items),
            );
            map
        })
    }
}
