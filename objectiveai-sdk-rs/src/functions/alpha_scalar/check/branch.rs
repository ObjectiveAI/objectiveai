//! Quality checks for alpha branch scalar functions.

use std::collections::{HashMap, HashSet};

use rand::Rng;
use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::functions::alpha_scalar::RemoteFunction;
use crate::functions::{CompiledTask, TaskExpression};

use crate::functions::check::check_description;
use crate::functions::check::check_input_schema;
use crate::functions::check::{
    ScalarOutputShape, check_scalar_distribution,
};
use crate::functions::check::{ScalarFieldsValidation, check_scalar_fields};
use crate::functions::check::{
    compile_and_validate_one_input, extract_task_input,
};
use crate::functions::check::example_inputs;

/// Validates quality requirements for an alpha branch scalar function.
///
/// The alpha type system already guarantees:
/// - All tasks are `scalar.function` or `placeholder.scalar.function`
/// - No `map` on any task
/// - Output expression is hardcoded (`Output`)
///
/// This checker validates the remaining runtime concerns.
pub fn check_alpha_branch_scalar_function(
    function: &RemoteFunction,
    children: Option<&HashMap<String, crate::functions::FullRemoteFunction>>,
    seed: Option<i64>,
) -> Result<(), String> {
    let (description, input_schema, tasks) = match function {
        RemoteFunction::Branch {
            description,
            input_schema,
            tasks,
        } => (description, input_schema, tasks),
        RemoteFunction::Leaf { .. } => {
            return Err(
                "AB01: Expected alpha.scalar.branch.function, got alpha.scalar.leaf.function"
                    .to_string(),
            );
        }
    };

    // Description
    check_description(description)?;

    // Transpile input_schema for permutation check
    let transpiled_input_schema =
        crate::functions::alpha_scalar::expression::scalar_function_input_schema::transpile(
            input_schema.clone(),
        );
    check_input_schema(&transpiled_input_schema)?;

    // Must have at least one task
    if tasks.is_empty() {
        return Err(
            "AB03: Functions must have at least one task".to_string(),
        );
    }

    // --- Transpile and run generate() loop ---
    let transpiled = function.clone().transpile();
    let input_schema = transpiled.input_schema();
    let task_count = tasks.len();

    let mut per_task_inputs: Vec<HashSet<String>> =
        vec![HashSet::new(); task_count];
    let mut per_task_skipped = vec![false; task_count];
    let mut seen_dist_tasks: HashSet<usize> = HashSet::new();
    let mut count = 0usize;

    let mut rng = match seed {
        Some(s) => StdRng::seed_from_u64(s as u64),
        None => StdRng::from_os_rng(),
    };

    let transpiled_children = children.map(|c| {
        c.iter().map(|(k, v)| (k.clone(), v.clone().transpile())).collect::<HashMap<_, _>>()
    });

    for ref input in example_inputs::generate_seeded(input_schema, StdRng::seed_from_u64(rng.random::<u64>())) {
        count += 1;
        let input_label = serde_json::to_string(input).unwrap_or_default();
        let compiled_tasks = compile_and_validate_one_input(
            &input_label,
            &transpiled,
            input,
            transpiled_children.as_ref(),
        )?;

        // Output expression distribution check (once per task)
        for (j, compiled_task) in compiled_tasks.iter().enumerate() {
            if let Some(CompiledTask::One(task)) = compiled_task {
                if seen_dist_tasks.insert(j) {
                    check_scalar_distribution(
                        j,
                        input,
                        task,
                        &ScalarOutputShape::Scalar,
                    )?;
                }
            }
        }

        // Track per-task input diversity
        for (j, compiled_task) in compiled_tasks.iter().enumerate() {
            let Some(compiled_task) = compiled_task else {
                per_task_skipped[j] = true;
                continue;
            };
            if let CompiledTask::One(task) = compiled_task {
                let key = extract_task_input(task);
                if !key.is_empty() {
                    per_task_inputs[j].insert(key);
                }
            }
        }
    }

    if count == 0 {
        return Err(
            "AB09: Failed to generate any example inputs from input_schema"
                .to_string(),
        );
    }

    // Post-loop: function input diversity check
    if count >= 2 {
        for (j, unique_inputs) in per_task_inputs.iter().enumerate() {
            let effective = unique_inputs.len()
                + if per_task_skipped[j] { 1 } else { 0 };
            if effective < 2 {
                return Err(format!(
                    "AB10: Task [{}]: task input is a fixed value — task inputs must \
                     be derived from the parent input, otherwise the score is useless",
                    j,
                ));
            }
        }
    }

    // Validate placeholder task fields
    let transpiled_tasks = match &transpiled {
        crate::functions::RemoteFunction::Scalar { tasks, .. } => tasks,
        _ => unreachable!(),
    };
    for (i, task) in transpiled_tasks.iter().enumerate() {
        if let TaskExpression::PlaceholderScalarFunction(psf) = task {
            check_scalar_fields(ScalarFieldsValidation {
                input_schema: psf.input_schema.clone(),
            }, seed)
            .map_err(|e| {
                format!(
                    "AB11: Task [{}]: placeholder scalar field validation failed: {}",
                    i, e
                )
            })?;
        }
    }

    Ok(())
}
