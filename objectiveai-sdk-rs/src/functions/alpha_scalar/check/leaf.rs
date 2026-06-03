//! Quality checks for alpha leaf scalar functions.

use std::collections::HashSet;

use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;

use crate::functions::alpha_scalar::RemoteFunction;
use crate::functions::{CompiledTask, Task};

use crate::functions::check::check_description;
use crate::functions::check::check_input_schema;
use crate::functions::check::compile_and_validate_one_input;
use crate::functions::check::example_inputs;
use crate::functions::check::{
    ModalityFlags, check_modality_coverage, collect_schema_modalities,
    collect_task_modalities,
};
use crate::functions::check::{ScalarOutputShape, check_scalar_distribution};

/// Validates quality requirements for an alpha leaf scalar function.
///
/// The alpha type system already guarantees:
/// - All tasks are `vector.completion` (no function/placeholder tasks)
/// - No `map` on any task
/// - Output expression is hardcoded (`TaskOutputWeightedSum`)
///
/// This checker validates the remaining runtime concerns.
pub fn check_alpha_leaf_scalar_function(
    function: &RemoteFunction,
    seed: Option<i64>,
) -> Result<(), String> {
    let (description, input_schema, tasks) = match function {
        RemoteFunction::Leaf {
            description,
            input_schema,
            tasks,
        } => (description, input_schema, tasks),
        RemoteFunction::Branch { .. } => {
            return Err(
                "AS01: Expected alpha.scalar.leaf.function, got alpha.scalar.branch.function"
                    .to_string(),
            );
        }
    };

    // Description length
    check_description(description)?;

    // Transpile input_schema for permutation check
    let transpiled_input_schema =
        crate::functions::alpha_scalar::expression::scalar_function_input_schema::transpile(
            input_schema.clone(),
        );
    check_input_schema(&transpiled_input_schema)?;

    // Must have at least one task
    if tasks.is_empty() {
        return Err("AS03: Functions must have at least one task".to_string());
    }

    // Pre-compile checks on alpha tasks
    for (i, task) in tasks.iter().enumerate() {
        let crate::functions::alpha_scalar::LeafTaskExpression::VectorCompletion(vc) = task;

        // Responses must have at least 2
        if vc.responses.len() < 2 {
            return Err(format!(
                "AS10: Task [{}]: responses must have at least 2 responses, found {}",
                i,
                vc.responses.len()
            ));
        }
    }

    // --- Transpile and run generate() loop ---
    let transpiled = function.clone().transpile();
    let input_schema = transpiled.input_schema();
    let task_count = tasks.len();

    let mut per_task_serialized: Vec<HashSet<String>> =
        vec![HashSet::new(); task_count];
    let mut per_task_skipped = vec![false; task_count];
    let mut seen_dist_tasks: HashSet<(usize, usize)> = HashSet::new();
    let mut count = 0usize;

    // Multimodal coverage tracking
    let mut schema_modalities: ModalityFlags = [false; 4];
    collect_schema_modalities(input_schema, &mut schema_modalities);
    let mut task_modalities: ModalityFlags = [false; 4];

    let mut rng = match seed {
        Some(s) => StdRng::seed_from_u64(s as u64),
        None => StdRng::from_os_rng(),
    };

    for ref input in example_inputs::generate_seeded(
        input_schema,
        StdRng::seed_from_u64(rng.random::<u64>()),
    ) {
        count += 1;
        let input_label = serde_json::to_string(input).unwrap_or_default();
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
                    check_scalar_distribution(
                        j,
                        input,
                        &Task::VectorCompletion(vc.clone()),
                        &ScalarOutputShape::VectorCompletion(
                            vc.responses.len(),
                        ),
                    )?;
                }
            }
        }

        // Track VC task diversity
        for (j, compiled_task) in compiled_tasks.iter().enumerate() {
            let Some(compiled_task) = compiled_task else {
                per_task_skipped[j] = true;
                continue;
            };
            if let CompiledTask::One(Task::VectorCompletion(vc)) = compiled_task
            {
                let key = serde_json::to_string(vc).unwrap_or_default();
                per_task_serialized[j].insert(key);
                collect_task_modalities(vc, &mut task_modalities);
            }
        }
    }

    if count == 0 {
        return Err(
            "AS18: Failed to generate any example inputs from input_schema"
                .to_string(),
        );
    }

    // Post-loop: VC task diversity check
    if count >= 2 {
        for (j, unique_tasks) in per_task_serialized.iter().enumerate() {
            let effective =
                unique_tasks.len() + if per_task_skipped[j] { 1 } else { 0 };
            if effective < 2 {
                return Err(format!(
                    "AS19: Task [{}]: task has fixed parameters — messages, tools, and/or \
                     responses must be derived from the parent input, otherwise \
                     the score is useless",
                    j,
                ));
            }
        }
    }

    // Multimodal coverage
    check_modality_coverage(&schema_modalities, &task_modalities, "AS20")?;

    Ok(())
}
