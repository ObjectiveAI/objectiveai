//! Input schema permutation validation.
//!
//! Ensures that the input schema produces at least 2 distinct example inputs,
//! otherwise diversity checks are meaningless.

use crate::functions::expression::InputSchema;

use super::example_inputs;

pub fn check_input_schema(input_schema: &InputSchema) -> Result<(), String> {
    let perms = example_inputs::permutations(input_schema);
    if perms < 2 {
        return Err(format!(
            "QI01: input_schema must produce at least 2 distinct example inputs, \
             got {}",
            perms
        ));
    }
    Ok(())
}
