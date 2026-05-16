//! Validation of scalar function fields (input_schema only).
//!
//! Verifies that the input schema produces enough diverse example inputs.

use rand::rngs::StdRng;
use rand::SeedableRng;
use serde::Deserialize;

use super::check_input_schema::check_input_schema;
use super::example_inputs;
use crate::functions::expression::InputSchema;
use schemars::JsonSchema;

/// The fields needed to validate a scalar function's input behavior.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[schemars(rename = "functions.check.ScalarFieldsValidation")]
pub struct ScalarFieldsValidation {
    pub input_schema: InputSchema,
}

/// Validate that the scalar fields are correct.
///
/// Generates example inputs from the `input_schema` and verifies that at least
/// one input can be produced.
pub fn check_scalar_fields(
    fields: ScalarFieldsValidation,
    seed: Option<i64>,
) -> Result<(), String> {
    check_input_schema(&fields.input_schema)?;

    let rng = match seed {
        Some(s) => StdRng::seed_from_u64(s as u64),
        None => StdRng::from_os_rng(),
    };

    let mut count = 0usize;
    for (_, ref _input) in
        example_inputs::generate_seeded(&fields.input_schema, rng).enumerate()
    {
        count += 1;
    }

    if count == 0 {
        return Err(
            "SF01: Failed to generate any example inputs from input_schema"
                .to_string(),
        );
    }

    Ok(())
}
