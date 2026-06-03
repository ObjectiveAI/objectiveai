use crate::functions;
use indexmap::IndexMap;

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

pub type ScalarFunctionInputValue =
    IndexMap<String, functions::expression::InputValue>;

pub mod scalar_function_input_value {
    use crate::functions;
    pub fn transpile(
        this: super::ScalarFunctionInputValue,
    ) -> functions::expression::InputValue {
        functions::expression::InputValue::Object(this)
    }
}
