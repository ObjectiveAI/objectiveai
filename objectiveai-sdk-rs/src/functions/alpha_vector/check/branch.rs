//! Quality checks for alpha branch vector functions.

use std::collections::{HashMap, HashSet};

use rand::Rng;
use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::functions::alpha_vector::{self, RemoteFunction};
use crate::functions::expression::{InputValue, Params, ParamsRef};
use crate::functions::{CompiledTask, Function, TaskExpression};

use crate::functions::check::check_description;
use crate::functions::check::check_input_schema;
use crate::functions::check::{
    VectorOutputShape, check_vector_distribution,
};
use crate::functions::check::{ScalarFieldsValidation, check_scalar_fields};
use crate::functions::check::{
    VectorFieldsValidation, check_vector_fields, check_vector_fields_for_input,
    random_subsets,
};
use crate::functions::check::{
    compile_and_validate_one_input, extract_task_input, extract_task_input_value,
};
use crate::functions::check::example_inputs;

/// Validates quality requirements for an alpha branch vector function.
///
/// The alpha type system already guarantees:
/// - No `map` on any task (transpile sets `map: None`)
/// - No `vector.completion` in branch tasks
/// - Input schema structurally enforces `{items, context?}`
/// - Vector fields (output_length, input_split, input_merge) are hardcoded Special expressions
///
/// This checker validates the remaining runtime concerns.
pub fn check_alpha_branch_vector_function(
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
                "AW01: Expected alpha.vector.branch.function, got alpha.vector.leaf.function"
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
    if tasks.is_empty() {
        return Err(
            "AW02: Functions must have at least one task".to_string(),
        );
    }

    // Composition rules: count scalar-like vs vector-like tasks
    let mut scalar_like_count: usize = 0;
    let mut vector_like_count: usize = 0;

    for task in tasks.iter() {
        match task {
            alpha_vector::BranchTaskExpression::ScalarFunction(_)
            | alpha_vector::BranchTaskExpression::PlaceholderScalarFunction(_) => {
                scalar_like_count += 1;
            }
            alpha_vector::BranchTaskExpression::VectorFunction(_)
            | alpha_vector::BranchTaskExpression::PlaceholderVectorFunction(_) => {
                vector_like_count += 1;
            }
        }
    }

    let total = scalar_like_count + vector_like_count;

    // If 1 task, must be vector-like
    if total == 1 && vector_like_count == 0 {
        return Err(
            "AW08: A branch vector function with a single task must use a \
             vector-like task (vector.function or placeholder.vector.function)"
                .to_string(),
        );
    }

    // At most 50% scalar-like
    if total > 1 && scalar_like_count * 2 > total {
        return Err(format!(
            "AW09: At most 50% of tasks in a branch vector function may be scalar-like, \
             found {}/{} ({:.0}%)",
            scalar_like_count,
            total,
            (scalar_like_count as f64 / total as f64) * 100.0
        ));
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
    let task_count = tasks.len();

    // Function input diversity tracking
    let mut per_task_inputs: Vec<HashSet<String>> =
        vec![HashSet::new(); task_count];
    // Mapped scalar per-index diversity
    let mut per_task_indexed: Vec<HashMap<usize, (usize, HashSet<String>)>> =
        vec![HashMap::new(); task_count];
    // Mapped scalar inputs not all equal
    let mut per_task_has_varying = vec![false; task_count];
    let mut per_task_is_mapped = vec![false; task_count];
    let mut per_task_skipped = vec![false; task_count];
    let mut seen_dist_tasks: HashSet<(usize, usize)> = HashSet::new();
    let mut count = 0usize;

    let transpiled_children = children.map(|c| {
        c.iter().map(|(k, v)| (k.clone(), v.clone().transpile())).collect::<HashMap<_, _>>()
    });

    let mut rng = match seed {
        Some(s) => StdRng::seed_from_u64(s as u64),
        None => StdRng::from_os_rng(),
    };

    for ref input in example_inputs::generate_seeded(transpiled_input_schema_ref, StdRng::seed_from_u64(rng.random::<u64>())) {
        count += 1;
        let input_label = serde_json::to_string(input).unwrap_or_default();

        // Vector fields validation
        check_vector_fields_for_input(&vector_fields, &input_label, input, &mut rng)?;

        // Compile and validate
        let compiled_tasks = compile_and_validate_one_input(
            &input_label,
            &transpiled,
            input,
            transpiled_children.as_ref(),
        )?;

        // Output expression distribution check (once per task+length pair)
        {
            let params = Params::Ref(ParamsRef {
                input,
                output: None,
                map: None,
                tasks_min: None,
                tasks_max: None,
                depth: None,
                name: None,
                spec: None,
            });
            let ol: usize = transpiled_output_length
                .clone()
                .compile_one::<u64>(&params)
                .unwrap_or(0) as usize;

            for (j, compiled_task) in compiled_tasks.iter().enumerate() {
                match compiled_task {
                    Some(CompiledTask::Many(tasks_vec)) => {
                        // Mapped scalar: key = (j, tasks.len())
                        let key = (j, tasks_vec.len());
                        if seen_dist_tasks.insert(key) {
                            if let Some(first) = tasks_vec.first() {
                                check_vector_distribution(
                                    j,
                                    input,
                                    first,
                                    &VectorOutputShape::MapScalar(
                                        tasks_vec.len(),
                                    ),
                                    ol,
                                )?;
                            }
                        }
                    }
                    Some(CompiledTask::One(task)) => {
                        // Unmapped vector: key = (j, output_length)
                        let key = (j, ol);
                        if seen_dist_tasks.insert(key) {
                            check_vector_distribution(
                                j,
                                input,
                                task,
                                &VectorOutputShape::Vector(ol as u64),
                                ol,
                            )?;
                        }
                    }
                    None => {}
                }
            }
        }

        // Track per-task input diversity + mapped scalar diversity
        for (j, compiled_task) in compiled_tasks.iter().enumerate() {
            let Some(compiled_task) = compiled_task else {
                per_task_skipped[j] = true;
                continue;
            };

            // Function input diversity
            let key = match compiled_task {
                CompiledTask::One(task) => extract_task_input(task),
                CompiledTask::Many(tasks_vec) => {
                    let inputs: Vec<_> = tasks_vec
                        .iter()
                        .filter_map(|t| extract_task_input_value(t))
                        .collect::<Vec<_>>();
                    serde_json::to_string(&inputs).unwrap_or_default()
                }
            };
            if !key.is_empty() {
                per_task_inputs[j].insert(key);
            }

            // Mapped scalar per-index diversity + not-all-equal
            if let CompiledTask::Many(tasks_vec) = compiled_task {
                per_task_is_mapped[j] = true;

                // Per-index diversity
                for (mi, task) in tasks_vec.iter().enumerate() {
                    if let Some(task_input) = extract_task_input_value(task) {
                        let k = serde_json::to_string(task_input)
                            .unwrap_or_default();
                        let entry = per_task_indexed[j]
                            .entry(mi)
                            .or_insert_with(|| (0, HashSet::new()));
                        entry.0 += 1;
                        entry.1.insert(k);
                    }
                }

                // Not all equal
                if !per_task_has_varying[j] && tasks_vec.len() >= 2 {
                    let first = extract_task_input_value(&tasks_vec[0])
                        .map(|v| {
                            serde_json::to_string(v).unwrap_or_default()
                        });
                    let has_different = tasks_vec[1..].iter().any(|t| {
                        extract_task_input_value(t).map(|v| {
                            serde_json::to_string(v).unwrap_or_default()
                        }) != first
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
                    "AW13: Merged input validation, input {}: input_split failed: {}",
                    input_label, e
                )
            })?
            .ok_or_else(|| {
                format!(
                    "AW14: Merged input validation, input {}: input_split returned None",
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
                            "AW15: Merged input validation, input {}, subset {:?}: \
                             input_merge failed: {}",
                            input_label, subset, e
                        )
                    })?
                    .ok_or_else(|| {
                        format!(
                            "AW16: Merged input validation, input {}, subset {:?}: \
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
                    transpiled_children.as_ref(),
                )?;
            }
        }
    }

    if count == 0 {
        return Err(
            "AW17: Failed to generate any example inputs from input_schema"
                .to_string(),
        );
    }

    // Post-loop diversity checks
    if count >= 2 {
        // Function input diversity
        for (j, unique_inputs) in per_task_inputs.iter().enumerate() {
            let effective = unique_inputs.len()
                + if per_task_skipped[j] { 1 } else { 0 };
            if effective < 2 {
                return Err(format!(
                    "AW18: Task [{}]: task input is a fixed value — task inputs must \
                     be derived from the parent input, otherwise the score is useless",
                    j,
                ));
            }
        }

        // Mapped scalar per-index diversity
        for (j, indexed) in per_task_indexed.iter().enumerate() {
            for (&mi, (occurrences, unique_inputs)) in indexed {
                let total = *occurrences
                    + if per_task_skipped[j] { 1 } else { 0 };
                if total <= 1 {
                    continue;
                }
                let effective = unique_inputs.len()
                    + if per_task_skipped[j] { 1 } else { 0 };
                if effective < 2 {
                    return Err(format!(
                        "AW19: Task [{}]: mapped input at index {} is a fixed value — \
                         mapped inputs must be derived from the parent input",
                        j, mi,
                    ));
                }
            }
        }

        // Mapped scalar inputs not all equal
        for (j, has_varying) in per_task_has_varying.iter().enumerate() {
            if !per_task_is_mapped[j] {
                continue;
            }
            if !has_varying && !per_task_skipped[j] {
                return Err(format!(
                    "AW20: Task [{}]: all mapped inputs are equal to each other for \
                     every example input — rankings are useless if every item \
                     is the same",
                    j,
                ));
            }
        }
    }

    // Validate placeholder task fields
    let transpiled_tasks = match &transpiled {
        crate::functions::RemoteFunction::Vector { tasks, .. } => tasks,
        _ => unreachable!(),
    };
    for (i, task) in transpiled_tasks.iter().enumerate() {
        match task {
            TaskExpression::PlaceholderScalarFunction(psf) => {
                check_scalar_fields(ScalarFieldsValidation {
                    input_schema: psf.input_schema.clone(),
                }, seed)
                .map_err(|e| {
                    format!(
                        "AW21: Task [{}]: placeholder scalar field validation failed: {}",
                        i, e
                    )
                })?;
            }
            TaskExpression::PlaceholderVectorFunction(pvf) => {
                check_vector_fields(VectorFieldsValidation {
                    input_schema: pvf.input_schema.clone(),
                    output_length: pvf.output_length.clone(),
                    input_split: pvf.input_split.clone(),
                    input_merge: pvf.input_merge.clone(),
                }, seed)
                .map_err(|e| {
                    format!(
                        "AW22: Task [{}]: placeholder vector field validation failed: {}",
                        i, e
                    )
                })?;
            }
            _ => {}
        }
    }

    Ok(())
}
