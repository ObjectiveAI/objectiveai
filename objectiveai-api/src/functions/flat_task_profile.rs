//! Flattened execution data for function execution.
//!
//! Transforms nested Function and Profile definitions into a flat structure
//! suitable for parallel execution.

use crate::ctx;
use futures::FutureExt;
use std::{pin::Pin, sync::Arc, task::Poll};

// ── Data types ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum FlatTaskProfile {
    Function(FunctionFlatTaskProfile),
    MapFunction(MapFunctionFlatTaskProfile),
    VectorCompletion(VectorCompletionFlatTaskProfile),
    MapVectorCompletion(MapVectorCompletionFlatTaskProfile),
    PlaceholderScalarFunction(PlaceholderScalarFunctionFlatTaskProfile),
    MapPlaceholderScalarFunction(MapPlaceholderScalarFunctionFlatTaskProfile),
    PlaceholderVectorFunction(PlaceholderVectorFunctionFlatTaskProfile),
    MapPlaceholderVectorFunction(MapPlaceholderVectorFunctionFlatTaskProfile),
}

impl FlatTaskProfile {
    pub fn len(&self) -> usize {
        match self {
            Self::Function(f) => f.len(),
            Self::MapFunction(mf) => mf.len(),
            Self::VectorCompletion(_) => 1,
            Self::MapVectorCompletion(mvc) => mvc.vector_completions.len(),
            Self::PlaceholderScalarFunction(_) => 1,
            Self::MapPlaceholderScalarFunction(mp) => mp.placeholders.len(),
            Self::PlaceholderVectorFunction(_) => 1,
            Self::MapPlaceholderVectorFunction(mp) => mp.placeholders.len(),
        }
    }

    pub fn task_index_len(&self) -> usize {
        match self {
            Self::Function(f) => f.task_index_len(),
            Self::MapFunction(mf) => mf.task_index_len(),
            Self::VectorCompletion(_) => 1,
            Self::MapVectorCompletion(mvc) => mvc.vector_completions.len().max(1),
            Self::PlaceholderScalarFunction(_) => 1,
            Self::MapPlaceholderScalarFunction(mp) => mp.placeholders.len().max(1),
            Self::PlaceholderVectorFunction(_) => 1,
            Self::MapPlaceholderVectorFunction(mp) => mp.placeholders.len().max(1),
        }
    }

    pub fn vector_completion_ftps(&self) -> Box<dyn Iterator<Item = &VectorCompletionFlatTaskProfile> + '_> {
        match self {
            Self::Function(f) => Box::new(
                f.tasks.iter()
                    .filter_map(|t| t.as_ref())
                    .flat_map(|t| t.vector_completion_ftps()),
            ),
            Self::MapFunction(mf) => Box::new(
                mf.functions.iter()
                    .flat_map(|f| f.tasks.iter().filter_map(|t| t.as_ref()).flat_map(|t| t.vector_completion_ftps())),
            ),
            Self::VectorCompletion(vc) => Box::new(std::iter::once(vc)),
            Self::MapVectorCompletion(mvc) => Box::new(mvc.vector_completions.iter()),
            _ => Box::new(std::iter::empty()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FunctionFlatTaskProfile {
    pub path: Vec<u64>,
    pub description: Option<String>,
    pub function_path: Option<objectiveai_sdk::RemotePath>,
    pub profile_path: Option<objectiveai_sdk::RemotePath>,
    pub input: objectiveai_sdk::functions::expression::InputValue,
    pub tasks: Vec<Option<FlatTaskProfile>>,
    pub profile: Vec<rust_decimal::Decimal>,
    pub r#type: FunctionType,
    pub task_output: Option<objectiveai_sdk::functions::expression::Expression>,
    pub invert_output: bool,
}

impl FunctionFlatTaskProfile {
    pub fn len(&self) -> usize {
        self.tasks.iter().map(|t| t.as_ref().map_or(1, |t| t.len())).sum()
    }
    pub fn task_index_len(&self) -> usize {
        self.tasks.iter().map(|t| t.as_ref().map_or(1, |t| t.task_index_len())).sum()
    }
    /// Returns cumulative task index offsets for each task.
    pub fn task_indices(&self) -> Vec<u64> {
        let mut indices = Vec::with_capacity(self.tasks.len());
        let mut offset = 0u64;
        for task in &self.tasks {
            indices.push(offset);
            offset += task.as_ref().map_or(1, |t| t.task_index_len()) as u64;
        }
        indices
    }
}

#[derive(Debug, Clone)]
pub enum FunctionType {
    Scalar,
    Vector {
        output_length: Option<u64>,
        input_split: Option<objectiveai_sdk::functions::expression::Expression>,
        input_merge: Option<objectiveai_sdk::functions::expression::Expression>,
    },
}

#[derive(Debug, Clone)]
pub struct MapFunctionFlatTaskProfile {
    pub path: Vec<u64>,
    pub functions: Vec<FunctionFlatTaskProfile>,
    pub task_output: objectiveai_sdk::functions::expression::Expression,
    pub invert_output: bool,
}

impl MapFunctionFlatTaskProfile {
    pub fn len(&self) -> usize { self.functions.iter().map(|f| f.len()).sum() }
    pub fn task_index_len(&self) -> usize {
        self.functions.iter().map(|f| f.task_index_len()).sum::<usize>().max(1)
    }
}

#[derive(Debug, Clone)]
pub struct VectorCompletionFlatTaskProfile {
    pub path: Vec<u64>,
    pub swarm: objectiveai_sdk::swarm::InlineSwarm,
    pub messages: Vec<objectiveai_sdk::agent::completions::message::Message>,
    pub responses: Vec<objectiveai_sdk::agent::completions::message::RichContent>,
    pub output: objectiveai_sdk::functions::expression::Expression,
    pub invert_output: bool,
}

impl VectorCompletionFlatTaskProfile {
    pub fn len(&self) -> usize { 1 }
    pub fn task_index_len(&self) -> usize { 1 }
}

#[derive(Debug, Clone)]
pub struct MapVectorCompletionFlatTaskProfile {
    pub path: Vec<u64>,
    pub vector_completions: Vec<VectorCompletionFlatTaskProfile>,
    pub task_output: objectiveai_sdk::functions::expression::Expression,
    pub invert_output: bool,
}

impl MapVectorCompletionFlatTaskProfile {
    pub fn len(&self) -> usize { self.vector_completions.len() }
    pub fn task_index_len(&self) -> usize { self.vector_completions.len().max(1) }
}

#[derive(Debug, Clone)]
pub struct PlaceholderScalarFunctionFlatTaskProfile {
    pub path: Vec<u64>,
    pub input: objectiveai_sdk::functions::expression::InputValue,
    pub output: objectiveai_sdk::functions::expression::Expression,
    pub invert_output: bool,
}

impl PlaceholderScalarFunctionFlatTaskProfile {
    pub fn len(&self) -> usize { 1 }
    pub fn task_index_len(&self) -> usize { 1 }
}

#[derive(Debug, Clone)]
pub struct MapPlaceholderScalarFunctionFlatTaskProfile {
    pub path: Vec<u64>,
    pub placeholders: Vec<PlaceholderScalarFunctionFlatTaskProfile>,
    pub task_output: objectiveai_sdk::functions::expression::Expression,
    pub invert_output: bool,
}

impl MapPlaceholderScalarFunctionFlatTaskProfile {
    pub fn len(&self) -> usize { self.placeholders.len() }
    pub fn task_index_len(&self) -> usize { self.placeholders.len().max(1) }
}

#[derive(Debug, Clone)]
pub struct PlaceholderVectorFunctionFlatTaskProfile {
    pub path: Vec<u64>,
    pub input: objectiveai_sdk::functions::expression::InputValue,
    pub output_length: u64,
    pub input_split: objectiveai_sdk::functions::expression::Expression,
    pub input_merge: objectiveai_sdk::functions::expression::Expression,
    pub output: objectiveai_sdk::functions::expression::Expression,
    pub invert_output: bool,
}

impl PlaceholderVectorFunctionFlatTaskProfile {
    pub fn len(&self) -> usize { 1 }
    pub fn task_index_len(&self) -> usize { 1 }
}

#[derive(Debug, Clone)]
pub struct MapPlaceholderVectorFunctionFlatTaskProfile {
    pub path: Vec<u64>,
    pub placeholders: Vec<PlaceholderVectorFunctionFlatTaskProfile>,
    pub task_output: objectiveai_sdk::functions::expression::Expression,
    pub invert_output: bool,
}

impl MapPlaceholderVectorFunctionFlatTaskProfile {
    pub fn len(&self) -> usize { self.placeholders.len() }
    pub fn task_index_len(&self) -> usize { self.placeholders.len().max(1) }
}

// ── Main entry ─────────────────────────────────────────────────────

/// Recursively builds a flattened task tree from a Function and Profile.
///
/// All fetching goes through the retrieve router's single codepath.
/// Function and profile are fetched concurrently. Nested function tasks
/// are recursively fetched in parallel.
pub async fn get_flat_task_profile<CTXEXT>(
    ctx: &ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
    mut path: Vec<u64>,
    function: objectiveai_sdk::functions::FullInlineFunctionOrRemoteCommitOptional,
    profile: objectiveai_sdk::functions::InlineProfileOrRemoteCommitOptional,
    input: objectiveai_sdk::functions::expression::InputValue,
    task_output: Option<objectiveai_sdk::functions::expression::Expression>,
    invert_output: bool,
    retrieve_router: Arc<
        crate::retrieval::retrieve::Router<
            impl crate::retrieval::retrieve::Client<CTXEXT> + Send + Sync + 'static,
            impl crate::retrieval::retrieve::Client<CTXEXT> + Send + Sync + 'static,
            impl crate::retrieval::retrieve::Client<CTXEXT> + Send + Sync + 'static,
            CTXEXT,
        >,
    >,
    mut ancestors: std::collections::HashSet<objectiveai_sdk::RemotePath>,
) -> Result<FunctionFlatTaskProfile, super::executions::Error>
where
    CTXEXT: Send + Sync + 'static,
{
    // 1. Fetch function + profile concurrently via router.
    let (function, function_path, profile, profile_path) = {
        let rr = retrieve_router.clone();
        let func_fut = async {
            match function {
                objectiveai_sdk::functions::FullInlineFunctionOrRemoteCommitOptional::Inline(inline) => {
                    let f = objectiveai_sdk::functions::Function::Inline(inline.transpile());
                    Ok::<_, super::executions::Error>((f, None))
                }
                objectiveai_sdk::functions::FullInlineFunctionOrRemoteCommitOptional::Remote(remote) => {
                    let resp = rr.endpoint_get_function(ctx, &remote).await
                        .map_err(super::executions::Error::FetchFunction)?;
                    let f = objectiveai_sdk::functions::Function::Remote(resp.inner.transpile());
                    Ok((f, Some(resp.path)))
                }
            }
        };
        let rr2 = retrieve_router.clone();
        let prof_fut = async {
            match profile {
                objectiveai_sdk::functions::InlineProfileOrRemoteCommitOptional::Inline(inline) => {
                    let p = objectiveai_sdk::functions::Profile::Inline(inline);
                    Ok((p, None))
                }
                objectiveai_sdk::functions::InlineProfileOrRemoteCommitOptional::Remote(remote) => {
                    let resp = rr2.endpoint_get_profile(ctx, &remote).await
                        .map_err(super::executions::Error::FetchProfile)?;
                    let p = objectiveai_sdk::functions::Profile::Remote(resp.inner);
                    Ok((p, Some(resp.path)))
                }
            }
        };
        let ((f, fp), (p, pp)) = tokio::try_join!(func_fut, prof_fut)?;
        (f, fp, p, pp)
    };

    // Cycle detection: check if this function is already in the ancestor chain.
    if let Some(ref fp) = function_path {
        if !ancestors.insert(fp.clone()) {
            return Err(super::executions::Error::CircularDependency(fp.clone()));
        }
    }

    // 2. Validate input.
    if let Some(schema) = function.input_schema() {
        if !schema.validate_input(&input) {
            return Err(super::executions::Error::InputSchemaMismatch);
        }
    }

    // 3. Extract weights + task profiles from profile.
    let function_tasks_len = function.tasks().len();

    struct TasksProfile {
        weights: Vec<rust_decimal::Decimal>,
        invert_flags: Vec<bool>,
        task_profiles: Vec<objectiveai_sdk::functions::TaskProfile>,
    }

    enum ResolvedProfile {
        Tasks(TasksProfile),
        Auto(objectiveai_sdk::swarm::InlineSwarmBase),
    }

    fn extract_tasks_profile(
        tasks: Vec<objectiveai_sdk::functions::TaskProfile>,
        weights: Option<objectiveai_sdk::Weights>,
        function_tasks_len: usize,
    ) -> Result<TasksProfile, super::executions::Error> {
        if tasks.len() != function_tasks_len {
            return Err(super::executions::Error::InvalidProfile(format!(
                "profile tasks length ({}) != function tasks length ({})",
                tasks.len(), function_tasks_len
            )));
        }
        let pairs = match weights {
            Some(w) => {
                if w.len() != function_tasks_len {
                    return Err(super::executions::Error::InvalidProfile(format!(
                        "profile weights length ({}) != function tasks length ({})",
                        w.len(), function_tasks_len
                    )));
                }
                w.to_weights_and_invert()
            }
            None => vec![(rust_decimal::Decimal::ONE, false); function_tasks_len],
        };
        let (weights, invert_flags) = pairs.into_iter().unzip();
        Ok(TasksProfile { weights, invert_flags, task_profiles: tasks })
    }

    let resolved_profile = match profile {
        objectiveai_sdk::functions::Profile::Remote(objectiveai_sdk::functions::RemoteProfile::Tasks(rp)) => {
            ResolvedProfile::Tasks(extract_tasks_profile(rp.inner.tasks, rp.inner.weights, function_tasks_len)?)
        }
        objectiveai_sdk::functions::Profile::Inline(objectiveai_sdk::functions::InlineProfile::Tasks(ip)) => {
            ResolvedProfile::Tasks(extract_tasks_profile(ip.tasks, ip.weights, function_tasks_len)?)
        }
        objectiveai_sdk::functions::Profile::Remote(objectiveai_sdk::functions::RemoteProfile::Auto(swarm_base)) => {
            ResolvedProfile::Auto(swarm_base.inner)
        }
        objectiveai_sdk::functions::Profile::Inline(objectiveai_sdk::functions::InlineProfile::Auto(swarm_base)) => {
            ResolvedProfile::Auto(swarm_base)
        }
    };

    // 4. Extract description + type.
    let description = function.description().map(str::to_owned);
    let r#type = match &function {
        objectiveai_sdk::functions::Function::Remote(objectiveai_sdk::functions::RemoteFunction::Scalar { .. }) => FunctionType::Scalar,
        objectiveai_sdk::functions::Function::Remote(objectiveai_sdk::functions::RemoteFunction::Vector { output_length, input_split, input_merge, .. }) => {
            let params = objectiveai_sdk::functions::expression::Params::Ref(objectiveai_sdk::functions::expression::ParamsRef {
                input: &input, output: None, map: None,
                tasks_min: None, tasks_max: None, depth: None, name: None, spec: None,
            });
            FunctionType::Vector {
                output_length: Some(output_length.clone().compile_one(&params)?),
                input_split: Some(input_split.clone()),
                input_merge: Some(input_merge.clone()),
            }
        }
        objectiveai_sdk::functions::Function::Inline(objectiveai_sdk::functions::InlineFunction::Scalar { .. }) => FunctionType::Scalar,
        objectiveai_sdk::functions::Function::Inline(objectiveai_sdk::functions::InlineFunction::Vector { input_split, input_merge, .. }) => {
            FunctionType::Vector { output_length: None, input_split: input_split.clone(), input_merge: input_merge.clone() }
        }
    };

    // 5. Compile tasks.
    let compiled_tasks = function.compile_tasks(&input)?;

    // Finalize weights.
    let (profile_weights, profile_invert_flags, task_profiles, auto_swarm) = match resolved_profile {
        ResolvedProfile::Tasks(tp) => (tp.weights, tp.invert_flags, Some(tp.task_profiles), None),
        ResolvedProfile::Auto(swarm) => {
            let n = compiled_tasks.len();
            let w = if n > 0 { rust_decimal::Decimal::ONE / rust_decimal::Decimal::from(n as u64) } else { rust_decimal::Decimal::ZERO };
            (vec![w; n], vec![false; n], None, Some(swarm))
        }
    };

    // 6. Build flat tasks concurrently.
    let mut flat_tasks_or_futs = Vec::with_capacity(compiled_tasks.len());
    let mut task_profiles_iter = task_profiles.map(|tp| tp.into_iter());

    for (i, task) in compiled_tasks.into_iter().enumerate() {
        let task_profile = task_profiles_iter.as_mut().map(|iter| iter.next().unwrap());
        let task = match task {
            Some(t) => t,
            None => { flat_tasks_or_futs.push(TaskFut::SkipTask); continue; }
        };
        let task_path = { path.push(i as u64); let p = path.clone(); path.pop(); p };

        match task {
            // ── Function tasks (recursive) ───────────────────────
            objectiveai_sdk::functions::CompiledTask::One(
                objectiveai_sdk::functions::Task::ScalarFunction(objectiveai_sdk::functions::ScalarFunctionTask { path, input, output })
            ) | objectiveai_sdk::functions::CompiledTask::One(
                objectiveai_sdk::functions::Task::VectorFunction(objectiveai_sdk::functions::VectorFunctionTask { path, input, output })
            ) => {
                let function_param = objectiveai_sdk::functions::FullInlineFunctionOrRemoteCommitOptional::Remote(
                    path.into(),
                );
                let profile_param = resolve_child_profile(&task_profile, &auto_swarm)?;
                let effective_invert = profile_invert_flags[i];
                flat_tasks_or_futs.push(TaskFut::FunctionTaskFut(Box::pin(
                    get_flat_task_profile(ctx, task_path, function_param, profile_param, input, Some(output), effective_invert, retrieve_router.clone(), ancestors.clone())
                )));
            }

            // ── Vector completion tasks ──────────────────────────
            objectiveai_sdk::functions::CompiledTask::One(
                objectiveai_sdk::functions::Task::VectorCompletion(vc_task)
            ) => {
                let swarm_base = resolve_vc_swarm_base(ctx, &task_profile, &auto_swarm, &retrieve_router).await?;
                flat_tasks_or_futs.push(TaskFut::VectorTaskFut(Box::pin(
                    resolve_vc_flat_task_profile(ctx, task_path, vc_task, swarm_base, profile_invert_flags[i], &retrieve_router)
                )));
            }

            // ── Placeholder scalar ───────────────────────────────
            objectiveai_sdk::functions::CompiledTask::One(
                objectiveai_sdk::functions::Task::PlaceholderScalarFunction(task)
            ) => {
                validate_placeholder_profile(&task_profile)?;
                flat_tasks_or_futs.push(TaskFut::Task(Some(FlatTaskProfile::PlaceholderScalarFunction(
                    PlaceholderScalarFunctionFlatTaskProfile {
                        path: task_path,
                        input: task.input,
                        output: task.output,
                        invert_output: profile_invert_flags[i],
                    },
                ))));
            }

            // ── Placeholder vector ───────────────────────────────
            objectiveai_sdk::functions::CompiledTask::One(
                objectiveai_sdk::functions::Task::PlaceholderVectorFunction(task)
            ) => {
                validate_placeholder_profile(&task_profile)?;
                let params = objectiveai_sdk::functions::expression::Params::Ref(
                    objectiveai_sdk::functions::expression::ParamsRef { input: &task.input, output: None, map: None, tasks_min: None, tasks_max: None, depth: None, name: None, spec: None },
                );
                let output_length = task.output_length.clone().compile_one(&params)?;
                flat_tasks_or_futs.push(TaskFut::Task(Some(FlatTaskProfile::PlaceholderVectorFunction(
                    PlaceholderVectorFunctionFlatTaskProfile {
                        path: task_path,
                        input: task.input,
                        output_length,
                        input_split: task.input_split,
                        input_merge: task.input_merge,
                        output: task.output,
                        invert_output: profile_invert_flags[i],
                    },
                ))));
            }

            // ── Mapped tasks ─────────────────────────────────────
            objectiveai_sdk::functions::CompiledTask::Many(tasks) => {
                let map_invert = profile_invert_flags[i];
                let map_output = tasks.first().map(|t| match t {
                    objectiveai_sdk::functions::Task::VectorCompletion(v) => v.output.clone(),
                    objectiveai_sdk::functions::Task::ScalarFunction(s) => s.output.clone(),
                    objectiveai_sdk::functions::Task::VectorFunction(v) => v.output.clone(),
                    objectiveai_sdk::functions::Task::PlaceholderScalarFunction(p) => p.output.clone(),
                    objectiveai_sdk::functions::Task::PlaceholderVectorFunction(p) => p.output.clone(),
                }).unwrap_or(objectiveai_sdk::functions::expression::Expression::JMESPath("output".to_string()));

                // Determine type from first task.
                let is_vc = matches!(tasks.first(), Some(objectiveai_sdk::functions::Task::VectorCompletion(_)));
                let is_fn = matches!(tasks.first(), Some(objectiveai_sdk::functions::Task::ScalarFunction(_) | objectiveai_sdk::functions::Task::VectorFunction(_)));
                let is_ps = matches!(tasks.first(), Some(objectiveai_sdk::functions::Task::PlaceholderScalarFunction(_)));
                let is_pv = matches!(tasks.first(), Some(objectiveai_sdk::functions::Task::PlaceholderVectorFunction(_)));

                if is_vc {
                    let mut vc_futs = Vec::with_capacity(tasks.len());
                    for (j, task) in tasks.into_iter().enumerate() {
                        let mut tp = task_path.clone(); tp.push(j as u64);
                        let vc_task = match task { objectiveai_sdk::functions::Task::VectorCompletion(t) => t, _ => unreachable!() };
                        let swarm_base = resolve_vc_swarm_base(ctx, &task_profile, &auto_swarm, &retrieve_router).await?;
                        vc_futs.push(resolve_vc_flat_task_profile(ctx, tp, vc_task, swarm_base, map_invert, &retrieve_router));
                    }
                    flat_tasks_or_futs.push(TaskFut::MapVectorTaskFut((
                        task_path, map_output, map_invert, futures::future::try_join_all(vc_futs),
                    )));
                } else if is_fn {
                    let mut futs = Vec::with_capacity(tasks.len());
                    for (j, task) in tasks.into_iter().enumerate() {
                        let mut tp = task_path.clone(); tp.push(j as u64);
                        let (path, input, _output) = match task {
                            objectiveai_sdk::functions::Task::ScalarFunction(t) => (t.path, t.input, t.output),
                            objectiveai_sdk::functions::Task::VectorFunction(t) => (t.path, t.input, t.output),
                            _ => unreachable!(),
                        };
                        let function_param = objectiveai_sdk::functions::FullInlineFunctionOrRemoteCommitOptional::Remote(
                            path.into(),
                        );
                        let profile_param = resolve_child_profile(&task_profile, &auto_swarm)?;
                        futs.push(get_flat_task_profile(ctx, tp, function_param, profile_param, input, None, false, retrieve_router.clone(), ancestors.clone()));
                    }
                    flat_tasks_or_futs.push(TaskFut::MapFunctionTaskFut((task_path, map_output, map_invert, futures::future::try_join_all(futs))));
                } else if is_ps {
                    validate_placeholder_profile(&task_profile)?;
                    let placeholders: Vec<_> = tasks.into_iter().enumerate().map(|(j, task)| {
                        let mut tp = task_path.clone(); tp.push(j as u64);
                        let t = match task { objectiveai_sdk::functions::Task::PlaceholderScalarFunction(t) => t, _ => unreachable!() };
                        PlaceholderScalarFunctionFlatTaskProfile { path: tp, input: t.input, output: t.output, invert_output: map_invert }
                    }).collect();
                    flat_tasks_or_futs.push(TaskFut::Task(Some(FlatTaskProfile::MapPlaceholderScalarFunction(
                        MapPlaceholderScalarFunctionFlatTaskProfile { path: task_path, placeholders, task_output: map_output, invert_output: map_invert },
                    ))));
                } else if is_pv {
                    validate_placeholder_profile(&task_profile)?;
                    let mut placeholders = Vec::with_capacity(tasks.len());
                    for (j, task) in tasks.into_iter().enumerate() {
                        let mut tp = task_path.clone(); tp.push(j as u64);
                        let t = match task { objectiveai_sdk::functions::Task::PlaceholderVectorFunction(t) => t, _ => unreachable!() };
                        let params = objectiveai_sdk::functions::expression::Params::Ref(
                            objectiveai_sdk::functions::expression::ParamsRef { input: &t.input, output: None, map: None, tasks_min: None, tasks_max: None, depth: None, name: None, spec: None },
                        );
                        let output_length = t.output_length.clone().compile_one(&params)?;
                        placeholders.push(PlaceholderVectorFunctionFlatTaskProfile {
                            path: tp, input: t.input, output_length,
                            input_split: t.input_split, input_merge: t.input_merge,
                            output: t.output, invert_output: map_invert,
                        });
                    }
                    flat_tasks_or_futs.push(TaskFut::Task(Some(FlatTaskProfile::MapPlaceholderVectorFunction(
                        MapPlaceholderVectorFunctionFlatTaskProfile { path: task_path, placeholders, task_output: map_output, invert_output: map_invert },
                    ))));
                }
            }
        }
    }

    // 7. Await all concurrent task resolutions.
    let tasks = futures::future::try_join_all(flat_tasks_or_futs).await?;

    Ok(FunctionFlatTaskProfile {
        path,
        description,
        function_path,
        profile_path,
        input,
        tasks,
        profile: profile_weights,
        r#type,
        task_output,
        invert_output,
    })
}

// ── Helpers ────────────────────────────────────────────────────────

/// Resolves the child profile for a nested function task.
fn resolve_child_profile(
    task_profile: &Option<objectiveai_sdk::functions::TaskProfile>,
    auto_swarm: &Option<objectiveai_sdk::swarm::InlineSwarmBase>,
) -> Result<objectiveai_sdk::functions::InlineProfileOrRemoteCommitOptional, super::executions::Error> {
    match task_profile {
        Some(objectiveai_sdk::functions::TaskProfile::Remote(path)) => {
            Ok(objectiveai_sdk::functions::InlineProfileOrRemoteCommitOptional::Remote(
                path.clone().into(),
            ))
        }
        Some(objectiveai_sdk::functions::TaskProfile::Inline(profile)) => {
            Ok(objectiveai_sdk::functions::InlineProfileOrRemoteCommitOptional::Inline(profile.clone()))
        }
        Some(objectiveai_sdk::functions::TaskProfile::Placeholder {}) => {
            Err(super::executions::Error::InvalidProfile(
                "expected function profile for function task, got Placeholder".to_string()
            ))
        }
        None => {
            // Auto mode — use the swarm as an auto profile.
            let swarm = auto_swarm.as_ref().expect("auto_swarm must be Some in auto mode");
            Ok(objectiveai_sdk::functions::InlineProfileOrRemoteCommitOptional::Inline(
                objectiveai_sdk::functions::InlineProfile::Auto(swarm.clone()),
            ))
        }
    }
}

/// Resolves a vector completion task's swarm and builds the flat task profile.
/// This is a named async function so both single and mapped VC tasks produce
/// the same future type (required by TaskFut's generic VFUT parameter).
async fn resolve_vc_flat_task_profile<CTXEXT>(
    ctx: &ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
    path: Vec<u64>,
    vc_task: objectiveai_sdk::functions::VectorCompletionTask,
    swarm_base: objectiveai_sdk::swarm::InlineSwarmBase,
    invert_output: bool,
    retrieve_router: &Arc<
        crate::retrieval::retrieve::Router<
            impl crate::retrieval::retrieve::Client<CTXEXT> + Send + Sync + 'static,
            impl crate::retrieval::retrieve::Client<CTXEXT> + Send + Sync + 'static,
            impl crate::retrieval::retrieve::Client<CTXEXT> + Send + Sync + 'static,
            CTXEXT,
        >,
    >,
) -> Result<VectorCompletionFlatTaskProfile, super::executions::Error>
where
    CTXEXT: Send + Sync + 'static,
{
    let swarm_param = objectiveai_sdk::swarm::InlineSwarmBaseOrRemoteCommitOptional::SwarmBase(swarm_base);
    let swarm = retrieve_router.get_swarm(ctx, swarm_param).await
        .map_err(|e| super::executions::Error::InvalidSwarm(e.message.to_string()))?
        .into_inline();
    Ok(VectorCompletionFlatTaskProfile {
        path,
        swarm,
        messages: vc_task.messages,
        responses: vc_task.responses,
        output: vc_task.output,
        invert_output,
    })
}

/// Extracts the swarm base for a vector completion task.
/// Supports inline auto profiles, auto mode fallback, and remote profile/swarm references.
async fn resolve_vc_swarm_base<CTXEXT>(
    ctx: &ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
    task_profile: &Option<objectiveai_sdk::functions::TaskProfile>,
    auto_swarm: &Option<objectiveai_sdk::swarm::InlineSwarmBase>,
    retrieve_router: &Arc<
        crate::retrieval::retrieve::Router<
            impl crate::retrieval::retrieve::Client<CTXEXT> + Send + Sync + 'static,
            impl crate::retrieval::retrieve::Client<CTXEXT> + Send + Sync + 'static,
            impl crate::retrieval::retrieve::Client<CTXEXT> + Send + Sync + 'static,
            CTXEXT,
        >,
    >,
) -> Result<objectiveai_sdk::swarm::InlineSwarmBase, super::executions::Error>
where
    CTXEXT: Send + Sync + 'static,
{
    match task_profile {
        Some(objectiveai_sdk::functions::TaskProfile::Inline(
            objectiveai_sdk::functions::InlineProfile::Auto(auto),
        )) => Ok(auto.clone()),
        Some(objectiveai_sdk::functions::TaskProfile::Inline(
            objectiveai_sdk::functions::InlineProfile::Tasks(_),
        )) => Err(super::executions::Error::InvalidProfile(
            "expected Auto (swarm) profile for vector completion task, got inline Tasks".to_string()
        )),
        Some(objectiveai_sdk::functions::TaskProfile::Remote(path)) => {
            let remote = objectiveai_sdk::functions::InlineProfileOrRemoteCommitOptional::Remote(
                path.clone().into(),
            );
            let profile = retrieve_router.get_profile(ctx, remote).await
                .map_err(super::executions::Error::FetchProfile)?;
            match profile {
                objectiveai_sdk::functions::Profile::Remote(objectiveai_sdk::functions::RemoteProfile::Auto(swarm_base)) => Ok(swarm_base.inner),
                objectiveai_sdk::functions::Profile::Remote(objectiveai_sdk::functions::RemoteProfile::Tasks(_)) => Err(super::executions::Error::InvalidProfile(
                    "expected Auto (swarm) profile for vector completion task, got remote Tasks".to_string()
                )),
                objectiveai_sdk::functions::Profile::Inline(objectiveai_sdk::functions::InlineProfile::Auto(swarm_base)) => Ok(swarm_base),
                objectiveai_sdk::functions::Profile::Inline(objectiveai_sdk::functions::InlineProfile::Tasks(_)) => Err(super::executions::Error::InvalidProfile(
                    "expected Auto (swarm) profile for vector completion task, got inline Tasks".to_string()
                )),
            }
        }
        Some(objectiveai_sdk::functions::TaskProfile::Placeholder {}) => Err(super::executions::Error::InvalidProfile(
            "expected Auto profile for vector completion task, got Placeholder".to_string()
        )),
        None => Ok(auto_swarm.as_ref().expect("auto_swarm must be Some in auto mode").clone()),
    }
}

fn validate_placeholder_profile(
    task_profile: &Option<objectiveai_sdk::functions::TaskProfile>,
) -> Result<(), super::executions::Error> {
    if let Some(tp) = task_profile {
        match tp {
            objectiveai_sdk::functions::TaskProfile::Placeholder {} => Ok(()),
            _ => Err(super::executions::Error::InvalidProfile(
                "expected Placeholder profile for placeholder task".to_string()
            )),
        }
    } else {
        Ok(())
    }
}

// ── TaskFut ────────────────────────────────────────────────────────

enum TaskFut<
    VFUT: Future<Output = Result<VectorCompletionFlatTaskProfile, super::executions::Error>>,
    FFUT: Future<Output = Result<FunctionFlatTaskProfile, super::executions::Error>>,
> {
    SkipTask,
    Task(Option<FlatTaskProfile>),
    VectorTaskFut(Pin<Box<VFUT>>),
    MapVectorTaskFut((
        Vec<u64>,
        objectiveai_sdk::functions::expression::Expression,
        bool,
        futures::future::TryJoinAll<VFUT>,
    )),
    FunctionTaskFut(Pin<Box<FFUT>>),
    MapFunctionTaskFut((
        Vec<u64>,
        objectiveai_sdk::functions::expression::Expression,
        bool,
        futures::future::TryJoinAll<FFUT>,
    )),
}

use std::future::Future;

impl<VFUT, FFUT> Future for TaskFut<VFUT, FFUT>
where
    VFUT: Future<Output = Result<VectorCompletionFlatTaskProfile, super::executions::Error>>,
    FFUT: Future<Output = Result<FunctionFlatTaskProfile, super::executions::Error>>,
{
    type Output = Result<Option<FlatTaskProfile>, super::executions::Error>;
    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        match self.get_mut() {
            TaskFut::SkipTask => Poll::Ready(Ok(None)),
            TaskFut::Task(task) => Poll::Ready(Ok(task.take())),
            TaskFut::VectorTaskFut(fut) => Pin::new(fut)
                .poll(cx)
                .map_ok(FlatTaskProfile::VectorCompletion)
                .map_ok(Some),
            TaskFut::MapVectorTaskFut((path, task_output, invert_output, futs)) => {
                Pin::new(futs).poll(cx).map_ok(|results| {
                    Some(FlatTaskProfile::MapVectorCompletion(
                        MapVectorCompletionFlatTaskProfile {
                            path: path.clone(),
                            vector_completions: results,
                            task_output: task_output.clone(),
                            invert_output: *invert_output,
                        },
                    ))
                })
            }
            TaskFut::FunctionTaskFut(fut) => Pin::new(fut)
                .poll(cx)
                .map_ok(FlatTaskProfile::Function)
                .map_ok(Some),
            TaskFut::MapFunctionTaskFut((path, task_output, invert_output, futs)) => {
                Pin::new(futs).poll(cx).map_ok(|results| {
                    Some(FlatTaskProfile::MapFunction(MapFunctionFlatTaskProfile {
                        path: path.clone(),
                        functions: results,
                        task_output: task_output.clone(),
                        invert_output: *invert_output,
                    }))
                })
            }
        }
    }
}

trait Pipe: Sized {
    fn pipe<F, R>(self, f: F) -> R where F: FnOnce(Self) -> R { f(self) }
}
impl<T> Pipe for T {}
