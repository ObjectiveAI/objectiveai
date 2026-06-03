//! Quality checks for alpha leaf vector functions.

use std::collections::{HashMap, HashSet};

use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;

use crate::functions::alpha_vector::RemoteFunction;
use crate::functions::check::check_description;
use crate::functions::check::check_input_schema;
use crate::functions::check::compile_and_validate_one_input;
use crate::functions::check::example_inputs;
use crate::functions::check::{
    ModalityFlags, check_modality_coverage, collect_schema_modalities,
    collect_task_modalities,
};
use crate::functions::check::{
    VectorFieldsValidation, check_vector_fields_for_input, random_subsets,
};
use crate::functions::check::{VectorOutputShape, check_vector_distribution};
use crate::functions::expression::InputValue;
use crate::functions::{CompiledTask, Function, Task};

/// Validates quality requirements for an alpha leaf vector function.
///
/// The alpha type system already guarantees:
/// - All tasks are `vector.completion` (no function/placeholder tasks)
/// - No `map` on leaf tasks
/// - Vector fields (output_length, input_split, input_merge) are hardcoded Special expressions
/// - Input schema structurally enforces `{items, context?}`
/// - Output expression is hardcoded (`Output`)
///
/// This checker validates the remaining runtime concerns.
pub fn check_alpha_leaf_vector_function(
    function: &RemoteFunction,
    seed: Option<i64>,
) -> Result<(), String> {
    let (description, input_schema, _tasks) = match function {
        RemoteFunction::Leaf {
            description,
            input_schema,
            tasks,
        } => (description, input_schema, tasks),
        RemoteFunction::Branch { .. } => {
            return Err(
                "AV01: Expected alpha.vector.leaf.function, got alpha.vector.branch.function"
                    .to_string(),
            );
        }
    };

    // Description
    check_description(description)?;

    // Transpile input_schema for permutation check
    let transpiled_input_schema = input_schema.clone().transpile();
    check_input_schema(&transpiled_input_schema)?;

    // Must have at least one task
    if _tasks.is_empty() {
        return Err("AV03: Functions must have at least one task".to_string());
    }

    // --- Transpile and run generate() loop ---
    let transpiled = function.clone().transpile();
    let (
        transpiled_input_schema_ref,
        transpiled_output_length,
        transpiled_input_split,
        transpiled_input_merge,
    ) = match &transpiled {
        crate::functions::RemoteFunction::Vector {
            input_schema,
            output_length,
            input_split,
            input_merge,
            ..
        } => (input_schema, output_length, input_split, input_merge),
        _ => unreachable!(),
    };

    let vector_fields = VectorFieldsValidation {
        input_schema: transpiled_input_schema_ref.clone(),
        output_length: transpiled_output_length.clone(),
        input_split: transpiled_input_split.clone(),
        input_merge: transpiled_input_merge.clone(),
    };
    let func_template = Function::Remote(transpiled.clone());
    let task_count = _tasks.len();

    // Response diversity tracking: per_task_indexed[j][i] = (occurrences, unique_values)
    let mut per_task_indexed: Vec<HashMap<usize, (usize, HashSet<String>)>> =
        vec![HashMap::new(); task_count];
    // Responses not all equal tracking
    let mut per_task_has_varying = vec![false; task_count];
    let mut per_task_skipped = vec![false; task_count];
    let mut seen_dist_tasks: HashSet<(usize, usize)> = HashSet::new();
    let mut count = 0usize;

    // Multimodal coverage tracking
    let mut schema_modalities: ModalityFlags = [false; 4];
    collect_schema_modalities(
        transpiled_input_schema_ref,
        &mut schema_modalities,
    );
    let mut task_modalities: ModalityFlags = [false; 4];

    let mut rng = match seed {
        Some(s) => StdRng::seed_from_u64(s as u64),
        None => StdRng::from_os_rng(),
    };

    for ref input in example_inputs::generate_seeded(
        transpiled_input_schema_ref,
        StdRng::seed_from_u64(rng.random::<u64>()),
    ) {
        count += 1;
        let input_label = serde_json::to_string(input).unwrap_or_default();

        // Vector fields validation
        check_vector_fields_for_input(
            &vector_fields,
            &input_label,
            input,
            &mut rng,
        )?;

        // Compile and validate
        let compiled_tasks = compile_and_validate_one_input(
            &input_label,
            &transpiled,
            input,
            None,
        )?;

        // Output expression distribution check (once per task+response_count)
        for (j, compiled_task) in compiled_tasks.iter().enumerate() {
            if let Some(CompiledTask::One(Task::VectorCompletion(vc))) =
                compiled_task
            {
                let key = (j, vc.responses.len());
                if seen_dist_tasks.insert(key) {
                    let ol = func_template
                        .clone()
                        .compile_output_length(input)
                        .ok()
                        .flatten()
                        .unwrap_or(0) as usize;
                    check_vector_distribution(
                        j,
                        input,
                        &Task::VectorCompletion(vc.clone()),
                        &VectorOutputShape::VectorCompletion(
                            vc.responses.len(),
                        ),
                        ol,
                    )?;
                }
            }
        }

        // Track response diversity and responses-not-all-equal
        for (j, compiled_task) in compiled_tasks.iter().enumerate() {
            let Some(compiled_task) = compiled_task else {
                per_task_skipped[j] = true;
                continue;
            };
            if let CompiledTask::One(Task::VectorCompletion(vc)) = compiled_task
            {
                collect_task_modalities(vc, &mut task_modalities);

                // Response diversity: per-index tracking
                for (ri, response) in vc.responses.iter().enumerate() {
                    let key =
                        serde_json::to_string(response).unwrap_or_default();
                    let entry = per_task_indexed[j]
                        .entry(ri)
                        .or_insert_with(|| (0, HashSet::new()));
                    entry.0 += 1;
                    entry.1.insert(key);
                }

                // Responses not all equal
                if !per_task_has_varying[j] && vc.responses.len() >= 2 {
                    let first = serde_json::to_string(&vc.responses[0])
                        .unwrap_or_default();
                    let has_different = vc.responses[1..].iter().any(|r| {
                        serde_json::to_string(r).unwrap_or_default() != first
                    });
                    if has_different {
                        per_task_has_varying[j] = true;
                    }
                }
            }
        }

        // Merged sub-inputs validation
        let splits = func_template
            .clone()
            .compile_input_split(input)
            .map_err(|e| {
                format!(
                    "AV09: Merged input validation, input {}: input_split failed: {}",
                    input_label, e
                )
            })?
            .ok_or_else(|| {
                format!(
                    "AV10: Merged input validation, input {}: input_split returned None",
                    input_label
                )
            })?;

        if splits.len() >= 2 {
            let subsets = random_subsets(splits.len(), 3, &mut rng);
            for subset in &subsets {
                let sub_splits: Vec<InputValue> =
                    subset.iter().map(|&idx| splits[idx].clone()).collect();
                let merge_input = InputValue::Array(sub_splits);
                let merged = func_template
                    .clone()
                    .compile_input_merge(&merge_input)
                    .map_err(|e| {
                        format!(
                            "AV11: Merged input validation, input {}, subset {:?}: \
                             input_merge failed: {}",
                            input_label, subset, e
                        )
                    })?
                    .ok_or_else(|| {
                        format!(
                            "AV12: Merged input validation, input {}, subset {:?}: \
                             input_merge returned None",
                            input_label, subset
                        )
                    })?;
                let merged_label =
                    serde_json::to_string(&merged).unwrap_or_default();
                compile_and_validate_one_input(
                    &merged_label,
                    &transpiled,
                    &merged,
                    None,
                )?;
            }
        }
    }

    if count == 0 {
        return Err(
            "AV15: Failed to generate any example inputs from input_schema"
                .to_string(),
        );
    }

    // Post-loop: response diversity check
    if count >= 2 {
        for (j, indexed) in per_task_indexed.iter().enumerate() {
            for (&ri, (occurrences, unique_values)) in indexed {
                let total =
                    *occurrences + if per_task_skipped[j] { 1 } else { 0 };
                if total <= 1 {
                    continue;
                }
                let effective = unique_values.len()
                    + if per_task_skipped[j] { 1 } else { 0 };
                if effective < 2 {
                    return Err(format!(
                        "AV16: Task [{}]: response at index {} is a fixed value — \
                         responses must be derived from an array in the input",
                        j, ri,
                    ));
                }
            }
        }

        // Responses not all equal check
        for (j, has_varying) in per_task_has_varying.iter().enumerate() {
            if !has_varying && !per_task_skipped[j] {
                return Err(format!(
                    "AV17: Task [{}]: all responses are equal to each other for every \
                     example input — rankings are useless if every item is the same",
                    j,
                ));
            }
        }
    }

    // Multimodal coverage
    check_modality_coverage(&schema_modalities, &task_modalities, "AV18")?;

    Ok(())
}
