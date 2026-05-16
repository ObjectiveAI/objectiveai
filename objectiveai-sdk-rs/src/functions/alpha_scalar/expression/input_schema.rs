use crate::functions;

pub type ScalarFunctionInputSchema = functions::expression::ObjectInputSchema;

pub mod scalar_function_input_schema {
    use crate::functions;
    pub fn transpile(
        this: super::ScalarFunctionInputSchema,
    ) -> functions::expression::InputSchema {
        functions::expression::InputSchema::Object(this)
    }
}
