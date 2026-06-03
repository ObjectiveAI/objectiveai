//! Validation of vector function fields (output_length, input_split, input_merge).
//!
//! Verifies that these expressions work correctly together via round-trip testing
//! against randomized example inputs.

use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;
use serde::Deserialize;

use super::check_input_schema::check_input_schema;
use super::example_inputs;
use crate::functions::expression::{Expression, InputSchema, InputValue};
use crate::functions::{Function, RemoteFunction};
use schemars::JsonSchema;

/// The 4 fields needed to validate a vector function's split/merge behavior.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[schemars(rename = "functions.check.VectorFieldsValidation")]
pub struct VectorFieldsValidation {
    pub input_schema: InputSchema,
    pub output_length: Expression,
    pub input_split: Expression,
    pub input_merge: Expression,
}

impl VectorFieldsValidation {
    /// Construct a minimal `Function` for compilation, cloning our expressions.
    fn to_function(&self) -> Function {
        Function::Remote(RemoteFunction::Vector {
            description: String::new(),
            input_schema: self.input_schema.clone(),
            tasks: vec![],
            output_length: self.output_length.clone(),
            input_split: self.input_split.clone(),
            input_merge: self.input_merge.clone(),
        })
    }
}

/// Validate that the vector fields work together correctly.
///
/// Generates diverse, randomized example inputs from the `input_schema`, then
/// validates each one via [`check_vector_fields_for_input`].
pub fn check_vector_fields(
    fields: VectorFieldsValidation,
    seed: Option<i64>,
) -> Result<(), String> {
    // Input schema permutations
    check_input_schema(&fields.input_schema)?;

    let mut rng = match seed {
        Some(s) => StdRng::seed_from_u64(s as u64),
        None => StdRng::from_os_rng(),
    };

    let mut count = 0usize;
    for ref input in example_inputs::generate_seeded(
        &fields.input_schema,
        StdRng::seed_from_u64(rng.random::<u64>()),
    ) {
        count += 1;
        let input_label = serde_json::to_string(input).unwrap_or_default();
        check_vector_fields_for_input(&fields, &input_label, input, &mut rng)?;
    }

    if count == 0 {
        return Err(
            "VF22: Failed to generate any example inputs from input_schema"
                .to_string(),
        );
    }

    Ok(())
}

/// Validates vector fields for a single input:
/// 1. Compiles `output_length` — must be > 0
/// 2. Compiles `input_split` — length must equal output_length
/// 3. Each split element must produce output_length = 1
/// 4. Merging all splits must reconstruct the original input
/// 5. Merging random subsets must produce output_length = subset size
pub(crate) fn check_vector_fields_for_input(
    fields: &VectorFieldsValidation,
    input_label: &str,
    input: &InputValue,
    rng: &mut impl Rng,
) -> Result<(), String> {
    // 1. Compile output_length
    let output_length = fields
        .to_function()
        .compile_output_length(input)
        .map_err(|e| {
            format!("VF01: Input {}: output_length compilation failed: {}", input_label, e)
        })?
        .ok_or_else(|| {
            format!(
                "VF02: Input {}: output_length returned None (not a vector function?)",
                input_label
            )
        })?;

    if output_length < 2 {
        return Err(format!(
            "VF03: Input {}: output_length must be > 1 for vector functions, got {}. Try setting `minItems` to 2 in the `input_schema`.",
            input_label, output_length,
        ));
    }

    // 2. Compile input_split
    let splits = fields
        .to_function()
        .compile_input_split(input)
        .map_err(|e| {
            format!(
                "VF04: Input {}: input_split compilation failed: {}",
                input_label, e
            )
        })?
        .ok_or_else(|| {
            format!("VF05: Input {}: input_split returned None", input_label)
        })?;

    if splits.len() as u64 != output_length {
        return Err(format!(
            "VF06: Input {}: input_split produced {} elements but output_length is {}",
            input_label,
            splits.len(),
            output_length,
        ));
    }

    // 3. Each split must produce output_length = 1
    for (j, split) in splits.iter().enumerate() {
        let split_len = fields
            .to_function()
            .compile_output_length(split)
            .map_err(|e| {
                format!(
                    "VF07: Input {}: output_length failed for split [{}]: {}",
                    input_label, j, e
                )
            })?
            .ok_or_else(|| {
                format!(
                    "VF08: Input {}: output_length returned None for split [{}]",
                    input_label, j
                )
            })?;

        if split_len != 1 {
            return Err(format!(
                "VF09: Input {}: split [{}] output_length must be 1, got {}.\n\nSplit: {}",
                input_label,
                j,
                split_len,
                serde_json::to_string(split).unwrap_or_default()
            ));
        }
    }

    // 4. Merge all splits — must equal original input
    let merge_input = InputValue::Array(splits.clone());
    let merged = fields
        .to_function()
        .compile_input_merge(&merge_input)
        .map_err(|e| {
            format!(
                "VF10: Input {}: input_merge compilation failed: {}",
                input_label, e
            )
        })?
        .ok_or_else(|| {
            format!("VF11: Input {}: input_merge returned None", input_label)
        })?;

    if !inputs_equal(input, &merged) {
        return Err(format!(
            "VF12: Input {}: merged input does not match original.\n\nOriginal: {}\n\nMerged: {}",
            input_label,
            serde_json::to_string(input).unwrap_or_default(),
            serde_json::to_string(&merged).unwrap_or_default()
        ));
    }

    // 5. Merged output_length equals original output_length
    let merged_len = fields
        .to_function()
        .compile_output_length(&merged)
        .map_err(|e| {
            format!(
                "VF13: Input {}: output_length failed for merged input: {}",
                input_label, e
            )
        })?
        .ok_or_else(|| {
            format!(
                "VF14: Input {}: output_length returned None for merged input",
                input_label
            )
        })?;

    if merged_len != output_length {
        return Err(format!(
            "VF15: Input {}: merged output_length ({}) != original output_length ({})",
            input_label, merged_len, output_length
        ));
    }

    // 6. Random subsets — merge and verify output_length = subset size
    //    and merged input satisfies input_schema constraints.
    let mut subsets = random_subsets(splits.len(), 5, rng);
    // Always test a 2-element subset deterministically so that
    // min_items violations are caught reliably.
    if splits.len() >= 3 {
        subsets.insert(0, vec![0, 1]);
    }
    for subset in &subsets {
        let sub_splits: Vec<InputValue> =
            subset.iter().map(|&idx| splits[idx].clone()).collect();
        let sub_merge_input = InputValue::Array(sub_splits);
        let sub_merged = fields
            .to_function()
            .compile_input_merge(&sub_merge_input)
            .map_err(|e| {
                format!(
                    "VF16: Input {}: input_merge failed for subset {:?}: {}",
                    input_label, subset, e
                )
            })?
            .ok_or_else(|| {
                format!(
                    "VF17: Input {}: input_merge returned None for subset {:?}",
                    input_label, subset
                )
            })?;

        let sub_merged_len = fields
            .to_function()
            .compile_output_length(&sub_merged)
            .map_err(|e| {
                format!(
                    "VF18: Input {}: output_length failed for merged subset {:?}: {}",
                    input_label, subset, e
                )
            })?
            .ok_or_else(|| {
                format!(
                    "VF19: Input {}: output_length returned None for merged subset {:?}",
                    input_label, subset
                )
            })?;

        if sub_merged_len as usize != subset.len() {
            return Err(format!(
                "VF20: Input {}: merged subset {:?} output_length is {}, expected {}",
                input_label,
                subset,
                sub_merged_len,
                subset.len()
            ));
        }

        // Merged subset must satisfy the input_schema constraints
        // (e.g., min_items). This ensures the function can execute
        // correctly with merged sub-inputs (used by swiss_system).
        validate_input_against_schema(
            &sub_merged,
            &fields.input_schema,
            "root",
        )
        .map_err(|e| {
            format!(
                "VF21: Input {}: merged subset {:?} violates input_schema: {}",
                input_label, subset, e
            )
        })?;
    }

    Ok(())
}

/// Validate that an input satisfies the schema's structural constraints.
/// Checks array min_items/max_items recursively through objects.
fn validate_input_against_schema(
    input: &InputValue,
    schema: &InputSchema,
    path: &str,
) -> Result<(), String> {
    match (input, schema) {
        (InputValue::Array(arr), InputSchema::Array(arr_schema)) => {
            if let Some(min) = arr_schema.min_items {
                if (arr.len() as u64) < min {
                    return Err(format!(
                        "VF23: {}: array has {} items but min_items is {}",
                        path,
                        arr.len(),
                        min
                    ));
                }
            }
            if let Some(max) = arr_schema.max_items {
                if (arr.len() as u64) > max {
                    return Err(format!(
                        "VF24: {}: array has {} items but max_items is {}",
                        path,
                        arr.len(),
                        max
                    ));
                }
            }
            for (i, item) in arr.iter().enumerate() {
                validate_input_against_schema(
                    item,
                    &arr_schema.items,
                    &format!("{}[{}]", path, i),
                )?;
            }
            Ok(())
        }
        (InputValue::Object(obj), InputSchema::Object(obj_schema)) => {
            for (key, prop_schema) in &obj_schema.properties {
                if let Some(value) = obj.get(key) {
                    validate_input_against_schema(
                        value,
                        prop_schema,
                        &format!("{}.{}", path, key),
                    )?;
                }
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// Deep equality check for Input values.
pub(crate) fn inputs_equal(a: &InputValue, b: &InputValue) -> bool {
    match (a, b) {
        (InputValue::String(a), InputValue::String(b)) => a == b,
        (InputValue::Integer(a), InputValue::Integer(b)) => a == b,
        (InputValue::Number(a), InputValue::Number(b)) => a == b,
        (InputValue::Boolean(a), InputValue::Boolean(b)) => a == b,
        (InputValue::Array(a), InputValue::Array(b)) => {
            a.len() == b.len()
                && a.iter().zip(b.iter()).all(|(x, y)| inputs_equal(x, y))
        }
        (InputValue::Object(a), InputValue::Object(b)) => {
            a.len() == b.len()
                && a.iter().all(|(ka, va)| {
                    b.get(ka).is_some_and(|vb| inputs_equal(va, vb))
                })
        }
        (InputValue::RichContentPart(a), InputValue::RichContentPart(b)) => {
            a == b
        }
        _ => false,
    }
}

/// Generate random subsets of indices for subset merge testing.
pub(crate) fn random_subsets(
    length: usize,
    count: usize,
    rng: &mut impl Rng,
) -> Vec<Vec<usize>> {
    if length < 2 {
        return vec![];
    }

    let mut result = Vec::new();

    for _ in 0..count {
        let size = rng.random_range(2..=length);
        let mut all_indices: Vec<usize> = (0..length).collect();

        // Fisher-Yates shuffle
        for i in (1..all_indices.len()).rev() {
            let j = rng.random_range(0..=i);
            all_indices.swap(i, j);
        }

        let mut subset: Vec<usize> =
            all_indices.into_iter().take(size).collect();
        subset.sort();
        subset.dedup();

        if subset.len() >= 2 {
            result.push(subset);
        }
    }

    result
}
