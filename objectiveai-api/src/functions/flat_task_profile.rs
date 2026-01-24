//! Flattened execution data for function execution.
//!
//! Transforms nested Function and Profile definitions into a flat structure suitable
//! for parallel execution. A Function defines the task structure and expressions,
//! while a Profile provides the weights for each task. This module combines both
//! into flattened executable tasks.

use crate::ctx;
use futures::FutureExt;
use std::{
    pin::Pin,
    sync::{Arc, LazyLock},
    task::Poll,
};

/// A flattened task ready for execution.
///
/// Combines Function structure with Profile weights into an executable node.
/// Can be a function (with nested tasks), a mapped array of functions, a vector
/// completion, or a mapped array of vector completions.
#[derive(Debug, Clone)]
pub enum FlatTaskProfile {
    /// A single function task with nested tasks.
    Function(FunctionFlatTaskProfile),
    /// Multiple function tasks from a mapped expression.
    MapFunction(MapFunctionFlatTaskProfile),
    /// A single vector completion task.
    VectorCompletion(VectorCompletionFlatTaskProfile),
    /// Multiple vector completion tasks from a mapped expression.
    MapVectorCompletion(MapVectorCompletionFlatTaskProfile),
}

impl FlatTaskProfile {
    /// Returns an iterator over all vector completion tasks.
    ///
    /// Recursively traverses function tasks to collect all leaf vector completions.
    pub fn vector_completion_ftps(
        &self,
    ) -> impl Iterator<Item = &VectorCompletionFlatTaskProfile> {
        enum Iter<'a> {
            Function(
                Box<
                    dyn Iterator<Item = &'a VectorCompletionFlatTaskProfile>
                        + 'a,
                >,
            ),
            MapFunction(
                Box<
                    dyn Iterator<Item = &'a VectorCompletionFlatTaskProfile>
                        + 'a,
                >,
            ),
            VectorCompletion(Option<&'a VectorCompletionFlatTaskProfile>),
            MapVectorCompletion(
                std::slice::Iter<'a, VectorCompletionFlatTaskProfile>,
            ),
        }
        impl<'a> Iterator for Iter<'a> {
            type Item = &'a VectorCompletionFlatTaskProfile;
            fn next(&mut self) -> Option<Self::Item> {
                match self {
                    Iter::Function(iter) => iter.next(),
                    Iter::MapFunction(iter) => iter.next(),
                    Iter::VectorCompletion(opt) => opt.take(),
                    Iter::MapVectorCompletion(iter) => iter.next(),
                }
            }
        }
        match self {
            FlatTaskProfile::Function(function) => Iter::Function(Box::new(
                function
                    .tasks
                    .iter()
                    .filter_map(|task| task.as_ref())
                    .flat_map(|task| task.vector_completion_ftps()),
            )),
            FlatTaskProfile::MapFunction(functions) => {
                Iter::MapFunction(Box::new(
                    functions
                        .functions
                        .iter()
                        .flat_map(|function| function.tasks.iter())
                        .filter_map(|task| task.as_ref())
                        .flat_map(|task| task.vector_completion_ftps()),
                ))
            }
            FlatTaskProfile::VectorCompletion(vector) => {
                Iter::VectorCompletion(Some(vector))
            }
            FlatTaskProfile::MapVectorCompletion(vectors) => {
                Iter::MapVectorCompletion(vectors.vector_completions.iter())
            }
        }
    }
    /// Returns the total number of leaf tasks (vector completions).
    pub fn len(&self) -> usize {
        match self {
            FlatTaskProfile::Function(function) => function.len(),
            FlatTaskProfile::MapFunction(functions) => functions.len(),
            FlatTaskProfile::VectorCompletion(vector) => vector.len(),
            FlatTaskProfile::MapVectorCompletion(vectors) => vectors.len(),
        }
    }

    /// Returns the number of task indices needed for output assembly.
    pub fn task_index_len(&self) -> usize {
        match self {
            FlatTaskProfile::Function(function) => function.task_index_len(),
            FlatTaskProfile::MapFunction(functions) => {
                functions.task_index_len()
            }
            FlatTaskProfile::VectorCompletion(vector) => {
                vector.task_index_len()
            }
            FlatTaskProfile::MapVectorCompletion(vectors) => {
                vectors.task_index_len()
            }
        }
    }
}

/// Multiple function tasks from a mapped expression.
///
/// Created when a task has a `map` index pointing to an input_maps sub-array.
/// Each element in that array produces one function instance.
#[derive(Debug, Clone)]
pub struct MapFunctionFlatTaskProfile {
    /// Path to this task in the Function tree (indices into tasks arrays).
    pub path: Vec<u64>,
    /// The individual flattened function tasks, one per element in the mapped array.
    pub functions: Vec<FunctionFlatTaskProfile>,
}

impl MapFunctionFlatTaskProfile {
    pub fn len(&self) -> usize {
        self.functions
            .iter()
            .map(FunctionFlatTaskProfile::len)
            .sum()
    }

    pub fn task_index_len(&self) -> usize {
        self.functions
            .iter()
            .map(FunctionFlatTaskProfile::task_index_len)
            .sum::<usize>()
            .max(1)
    }
}

/// A flattened function task ready for execution.
///
/// Combines a Function definition with its corresponding Profile. Contains the
/// compiled input, nested tasks with their weights, and the output expression.
#[derive(Debug, Clone)]
pub struct FunctionFlatTaskProfile {
    /// Path to this task in the Function tree (indices into tasks arrays).
    pub path: Vec<u64>,
    /// Full Function ID (owner, repository, commit) if from GitHub.
    pub full_function_id: Option<(String, String, String)>,
    /// Full Profile ID (owner, repository, commit) if from GitHub.
    pub full_profile_id: Option<(String, String, String)>,
    /// Description from the Function definition.
    pub description: Option<String>,
    /// The compiled input for this Function.
    pub input: objectiveai::functions::expression::Input,
    /// The flattened child tasks (None if task was skipped).
    pub tasks: Vec<Option<FlatTaskProfile>>,
    /// The output expression for computing the final score.
    pub output: objectiveai::functions::expression::Expression,
    /// The Function type (scalar or vector).
    pub r#type: FunctionType,
}

impl FunctionFlatTaskProfile {
    pub fn len(&self) -> usize {
        self.tasks
            .iter()
            .map(|task| task.as_ref().map_or(1, |task| task.len()))
            .sum()
    }

    pub fn task_index_len(&self) -> usize {
        let mut len = 0;
        for task in &self.tasks {
            len += if let Some(task) = task {
                task.task_index_len()
            } else {
                1
            };
        }
        len
    }

    pub fn task_indices(&self) -> Vec<u64> {
        let mut indices = Vec::with_capacity(self.tasks.len());
        let mut current_index = 0u64;
        for task in &self.tasks {
            indices.push(current_index);
            current_index += if let Some(task) = task {
                task.task_index_len()
            } else {
                1
            } as u64;
        }
        indices
    }
}

/// The type of a Function's output.
#[derive(Debug, Clone, Copy)]
pub enum FunctionType {
    /// Produces a single score in [0, 1].
    Scalar,
    /// Produces a vector of scores that sums to ~1.
    Vector {
        /// Expected output length, if known from output_length expression.
        output_length: Option<u64>,
    },
}

/// Multiple vector completion tasks from a mapped expression.
///
/// Created when a vector completion task has a `map` index. Each element in the
/// mapped array produces one vector completion instance.
#[derive(Debug, Clone)]
pub struct MapVectorCompletionFlatTaskProfile {
    /// Path to this task in the Function tree (indices into tasks arrays).
    pub path: Vec<u64>,
    /// The individual flattened vector completion tasks.
    pub vector_completions: Vec<VectorCompletionFlatTaskProfile>,
}

impl MapVectorCompletionFlatTaskProfile {
    pub fn len(&self) -> usize {
        self.vector_completions.len()
    }

    pub fn task_index_len(&self) -> usize {
        self.vector_completions.len().max(1)
    }
}

/// A flattened vector completion task ready for execution.
///
/// The leaf task type. Contains everything needed to run a vector completion:
/// the Ensemble of LLMs, their weights from the Profile, and the compiled
/// messages/responses.
#[derive(Debug, Clone)]
pub struct VectorCompletionFlatTaskProfile {
    /// Path to this task in the Function tree (indices into tasks arrays).
    pub path: Vec<u64>,
    /// The Ensemble configuration with LLMs and their settings.
    pub ensemble: objectiveai::ensemble::EnsembleBase,
    /// The weights for each LLM in the Ensemble (from the Profile).
    pub profile: Vec<rust_decimal::Decimal>,
    /// The compiled messages for the vector completion.
    pub messages: Vec<objectiveai::chat::completions::request::Message>,
    /// Optional tools for the vector completion (read-only context).
    pub tools: Option<Vec<objectiveai::chat::completions::request::Tool>>,
    /// The compiled response options the LLMs will vote on.
    pub responses: Vec<objectiveai::chat::completions::request::RichContent>,
}

impl VectorCompletionFlatTaskProfile {
    pub fn len(&self) -> usize {
        1
    }

    pub fn task_index_len(&self) -> usize {
        1
    }
}

/// Parameter for specifying a function source.
#[derive(Debug, Clone)]
pub enum FunctionParam {
    /// Function to fetch from GitHub by owner/repository/commit.
    Remote {
        owner: String,
        repository: String,
        commit: Option<String>,
    },
    /// Already-fetched or inline function definition.
    FetchedOrInline {
        full_id: Option<(String, String, String)>,
        function: objectiveai::functions::Function,
    },
}

/// Parameter for specifying a profile source.
#[derive(Debug, Clone)]
pub enum ProfileParam {
    /// Profile to fetch from GitHub by owner/repository/commit.
    Remote {
        owner: String,
        repository: String,
        commit: Option<String>,
    },
    /// Already-fetched or inline profile definition.
    FetchedOrInline {
        full_id: Option<(String, String, String)>,
        profile: objectiveai::functions::Profile,
    },
}

/// Recursively builds a flattened task from a Function and Profile.
///
/// Fetches any remote Functions/Profiles/Ensembles, compiles task expressions
/// with the input, and validates that the Profile structure matches the Function.
/// The result is a flat tree of tasks ready for parallel execution.
pub async fn get_flat_task_profile<CTXEXT>(
    ctx: ctx::Context<CTXEXT>,
    mut path: Vec<u64>,
    function: FunctionParam,
    profile: ProfileParam,
    input: objectiveai::functions::expression::Input,
    function_fetcher: Arc<
        impl super::function_fetcher::Fetcher<CTXEXT> + Send + Sync + 'static,
    >,
    profile_fetcher: Arc<
        impl super::profile_fetcher::Fetcher<CTXEXT> + Send + Sync + 'static,
    >,
    ensemble_fetcher: Arc<
        crate::ensemble::fetcher::CachingFetcher<
            CTXEXT,
            impl crate::ensemble::fetcher::Fetcher<CTXEXT>
            + Send
            + Sync
            + 'static,
        >,
    >,
) -> Result<super::FunctionFlatTaskProfile, super::executions::Error>
where
    CTXEXT: Send + Sync + 'static,
{
    static EMPTY_TASKS: LazyLock<
        Vec<Option<objectiveai::functions::expression::TaskOutput>>,
    > = LazyLock::new(|| Vec::new());

    // fetch function and profile if needed
    let (function_full_id, function, profile_full_id, profile): (
        Option<(String, String, String)>,
        objectiveai::functions::Function,
        Option<(String, String, String)>,
        objectiveai::functions::Profile,
    ) = match (function, profile) {
        (
            FunctionParam::Remote {
                owner: fowner,
                repository: frepository,
                commit: fcommit,
            },
            ProfileParam::Remote {
                owner: powner,
                repository: prepository,
                commit: pcommit,
            },
        ) => {
            let ((function, fcommit), (profile, pcommit)) = tokio::try_join!(
                function_fetcher
                    .fetch(
                        ctx.clone(),
                        &fowner,
                        &frepository,
                        fcommit.as_deref()
                    )
                    .map(|result| match result {
                        Ok(Some(function)) => {
                            Ok((function.inner, function.commit))
                        }
                        Ok(_) =>
                            Err(super::executions::Error::FunctionNotFound),
                        Err(e) =>
                            Err(super::executions::Error::FetchFunction(e)),
                    }),
                profile_fetcher
                    .fetch(
                        ctx.clone(),
                        &powner,
                        &prepository,
                        pcommit.as_deref(),
                    )
                    .map(|result| match result {
                        Ok(Some(profile)) => {
                            Ok((profile.inner, profile.commit))
                        }
                        Ok(_) => Err(super::executions::Error::ProfileNotFound),
                        Err(e) =>
                            Err(super::executions::Error::FetchProfile(e)),
                    }),
            )?;
            (
                Some((fowner.to_owned(), frepository.to_owned(), fcommit)),
                objectiveai::functions::Function::Remote(function),
                Some((powner, prepository, pcommit)),
                objectiveai::functions::Profile::Remote(profile),
            )
        }
        (
            FunctionParam::Remote {
                owner: fowner,
                repository: frepository,
                commit: fcommit,
            },
            ProfileParam::FetchedOrInline {
                full_id: pfull_id,
                profile,
            },
        ) => {
            let (function, fcommit) = match function_fetcher
                .fetch(ctx.clone(), &fowner, &frepository, fcommit.as_deref())
                .await
            {
                Ok(Some(function)) => Ok((function.inner, function.commit)),
                Ok(_) => Err(super::executions::Error::FunctionNotFound),
                Err(e) => Err(super::executions::Error::FetchFunction(e)),
            }?;
            (
                Some((fowner, frepository, fcommit)),
                objectiveai::functions::Function::Remote(function),
                pfull_id,
                profile,
            )
        }
        (
            FunctionParam::FetchedOrInline {
                full_id: ffull_id,
                function,
            },
            ProfileParam::Remote {
                owner: powner,
                repository: prepository,
                commit: pcommit,
            },
        ) => {
            let (profile, pcommit) = match profile_fetcher
                .fetch(ctx.clone(), &powner, &prepository, pcommit.as_deref())
                .await
            {
                Ok(Some(profile)) => Ok((profile.inner, profile.commit)),
                Ok(_) => Err(super::executions::Error::ProfileNotFound),
                Err(e) => Err(super::executions::Error::FetchProfile(e)),
            }?;
            (
                ffull_id,
                function,
                Some((powner, prepository, pcommit)),
                objectiveai::functions::Profile::Remote(profile),
            )
        }
        (
            FunctionParam::FetchedOrInline {
                full_id: ffull_id,
                function,
            },
            ProfileParam::FetchedOrInline {
                full_id: pfull_id,
                profile,
            },
        ) => (ffull_id, function, pfull_id, profile),
    };

    // validate input against input_schema
    if let Some(input_schema) = function.input_schema() {
        if !input_schema.validate_input(&input) {
            return Err(super::executions::Error::InputSchemaMismatch);
        }
    }

    // validate profile length
    if match &profile {
        objectiveai::functions::Profile::Remote(rp) => rp.tasks.len(),
        objectiveai::functions::Profile::Inline(ip) => ip.tasks.len(),
    } != function.tasks().len()
    {
        return Err(super::executions::Error::InvalidProfile);
    }

    // take description
    let description = function.description().map(str::to_owned);

    // take output
    let output = function.output().clone();

    // take type, compile output_length if needed
    let r#type = match function {
        objectiveai::functions::Function::Remote(
            objectiveai::functions::RemoteFunction::Scalar { .. },
        ) => FunctionType::Scalar,
        objectiveai::functions::Function::Remote(
            objectiveai::functions::RemoteFunction::Vector {
                ref output_length,
                ..
            },
        ) => {
            let params = objectiveai::functions::expression::Params::Ref(
                objectiveai::functions::expression::ParamsRef {
                    input: &input,
                    tasks: &EMPTY_TASKS,
                    map: None,
                },
            );
            FunctionType::Vector {
                output_length: Some(
                    output_length.clone().compile_one(&params)?,
                ),
            }
        }
        objectiveai::functions::Function::Inline(
            objectiveai::functions::InlineFunction::Scalar { .. },
        ) => FunctionType::Scalar,
        objectiveai::functions::Function::Inline(
            objectiveai::functions::InlineFunction::Vector { .. },
        ) => FunctionType::Vector {
            output_length: None,
        },
    };

    // compile function tasks
    let tasks = function.compile_tasks(&input)?;

    // initialize flat tasks / futs vector
    let mut flat_tasks_or_futs = Vec::with_capacity(tasks.len());

    // iterate through tasks
    for (i, (task, profile)) in tasks
        .into_iter()
        .zip(match profile {
            objectiveai::functions::Profile::Remote(rp) => {
                either::Either::Left(rp.tasks.into_iter())
            }
            objectiveai::functions::Profile::Inline(ip) => {
                either::Either::Right(ip.tasks.into_iter())
            }
        })
        .enumerate()
    {
        // if skip, push None to flat tasks
        let task = match task {
            Some(task) => task,
            None => {
                flat_tasks_or_futs.push(TaskFut::SkipTask);
                continue;
            }
        };

        // task path
        let task_path = {
            path.push(i as u64);
            let p = path.clone();
            path.pop();
            p
        };

        // switch by task type
        match task {
            objectiveai::functions::CompiledTask::One(
                objectiveai::functions::Task::ScalarFunction(
                    objectiveai::functions::ScalarFunctionTask {
                        owner,
                        repository,
                        commit,
                        input,
                    },
                )
                | objectiveai::functions::Task::VectorFunction(
                    objectiveai::functions::VectorFunctionTask {
                        owner,
                        repository,
                        commit,
                        input,
                    },
                ),
            ) => {
                flat_tasks_or_futs.push(TaskFut::FunctionTaskFut(Box::pin(
                    get_flat_task_profile(
                        ctx.clone(),
                        task_path,
                        FunctionParam::Remote {
                            owner,
                            repository,
                            commit: Some(commit),
                        },
                        match profile {
                            objectiveai::functions::TaskProfile::RemoteFunction {
                                owner,
                                repository,
                                commit,
                            } => ProfileParam::Remote {
                                owner,
                                repository,
                                commit,
                            },
                            objectiveai::functions::TaskProfile::InlineFunction(
                                profile,
                            ) => ProfileParam::FetchedOrInline {
                                full_id: None,
                                profile: objectiveai::functions::Profile::Inline(
                                    profile,
                                ),
                            },
                            _ => return Err(super::executions::Error::InvalidProfile),
                        },
                        input,
                        function_fetcher.clone(),
                        profile_fetcher.clone(),
                        ensemble_fetcher.clone(),
                    )
                )));
            }
            objectiveai::functions::CompiledTask::One(
                objectiveai::functions::Task::VectorCompletion(task),
            ) => {
                let (ensemble, profile) = match profile {
                    objectiveai::functions::TaskProfile::VectorCompletion {
                        ensemble,
                        profile,
                    } => (ensemble, profile),
                    _ => return Err(super::executions::Error::InvalidProfile),
                };
                flat_tasks_or_futs.push(TaskFut::VectorTaskFut(Box::pin(
                    get_vector_completion_flat_task_profile(
                        ctx.clone(),
                        task_path,
                        task,
                        ensemble,
                        profile,
                        ensemble_fetcher.clone(),
                    ),
                )));
            }
            objectiveai::functions::CompiledTask::Many(tasks)
                if tasks.len() == 0 =>
            {
                flat_tasks_or_futs.push(TaskFut::Task(Some(
                    FlatTaskProfile::MapVectorCompletion(
                        MapVectorCompletionFlatTaskProfile {
                            path: task_path,
                            vector_completions: Vec::new(),
                        },
                    ),
                )));
            }
            objectiveai::functions::CompiledTask::Many(tasks) => {
                let vector_completions = match &tasks[0] {
                    objectiveai::functions::Task::VectorCompletion(_) => true,
                    _ => false,
                };
                if vector_completions {
                    let mut futs = Vec::with_capacity(tasks.len());
                    for (j, task) in tasks.into_iter().enumerate() {
                        let mut task_path = task_path.clone();
                        task_path.push(j as u64);
                        let (ensemble, profile) = match &profile {
                            objectiveai::functions::TaskProfile::VectorCompletion {
                                ensemble,
                                profile,
                            } => (ensemble.clone(), profile.clone()),
                            _ => return Err(super::executions::Error::InvalidProfile),
                        };
                        futs.push(get_vector_completion_flat_task_profile(
                            ctx.clone(),
                            task_path,
                            match task {
                                objectiveai::functions::Task::VectorCompletion(
                                    vc_task,
                                ) => vc_task,
                                _ => unreachable!(),
                            },
                            ensemble,
                            profile,
                            ensemble_fetcher.clone(),
                        ));
                    }
                    flat_tasks_or_futs.push(TaskFut::MapVectorTaskFut((
                        task_path,
                        futures::future::try_join_all(futs),
                    )));
                } else {
                    let mut futs = Vec::with_capacity(tasks.len());
                    for (j, task) in tasks.into_iter().enumerate() {
                        let mut task_path = task_path.clone();
                        task_path.push(j as u64);
                        futs.push(get_flat_task_profile(
                            ctx.clone(),
                            task_path,
                            FunctionParam::Remote {
                                owner: match &task {
                                    objectiveai::functions::Task::ScalarFunction(
                                        sf_task,
                                    ) => sf_task.owner.clone(),
                                    objectiveai::functions::Task::VectorFunction(
                                        vf_task,
                                    ) => vf_task.owner.clone(),
                                    _ => unreachable!(),
                                },
                                repository: match &task {
                                    objectiveai::functions::Task::ScalarFunction(
                                        sf_task,
                                    ) => sf_task.repository.clone(),
                                    objectiveai::functions::Task::VectorFunction(
                                        vf_task,
                                    ) => vf_task.repository.clone(),
                                    _ => unreachable!(),
                                },
                                commit: Some(match &task {
                                    objectiveai::functions::Task::ScalarFunction(
                                        sf_task,
                                    ) => sf_task.commit.clone(),
                                    objectiveai::functions::Task::VectorFunction(
                                        vf_task,
                                    ) => vf_task.commit.clone(),
                                    _ => unreachable!(),
                                }),
                            },
                            match &profile {
                                objectiveai::functions::TaskProfile::RemoteFunction {
                                    owner,
                                    repository,
                                    commit,
                                } => ProfileParam::Remote {
                                    owner: owner.clone(),
                                    repository: repository.clone(),
                                    commit: commit.clone(),
                                },
                                objectiveai::functions::TaskProfile::InlineFunction(
                                    profile,
                                ) => ProfileParam::FetchedOrInline {
                                    full_id: None,
                                    profile: objectiveai::functions::Profile::Inline(
                                        profile.clone(),
                                    ),
                                },
                                _ => return Err(super::executions::Error::InvalidProfile),
                            },
                            match &task {
                                objectiveai::functions::Task::ScalarFunction(
                                    sf_task,
                                ) => sf_task.input.clone(),
                                objectiveai::functions::Task::VectorFunction(
                                    vf_task,
                                ) => vf_task.input.clone(),
                                _ => unreachable!(),
                            },
                            function_fetcher.clone(),
                            profile_fetcher.clone(),
                            ensemble_fetcher.clone(),
                        ));
                    }
                    flat_tasks_or_futs.push(TaskFut::MapFunctionTaskFut((
                        task_path,
                        futures::future::try_join_all(futs),
                    )));
                }
            }
        }
    }

    // await all futs
    let tasks = futures::future::try_join_all(flat_tasks_or_futs).await?;

    // return flat function task
    Ok(super::FunctionFlatTaskProfile {
        path,
        description,
        full_function_id: function_full_id,
        full_profile_id: profile_full_id,
        input,
        tasks,
        output,
        r#type,
    })
}

async fn get_vector_completion_flat_task_profile<CTXEXT>(
    ctx: ctx::Context<CTXEXT>,
    path: Vec<u64>,
    task: objectiveai::functions::VectorCompletionTask,
    ensemble: objectiveai::vector::completions::request::Ensemble,
    profile: Vec<rust_decimal::Decimal>,
    ensemble_fetcher: Arc<
        crate::ensemble::fetcher::CachingFetcher<
            CTXEXT,
            impl crate::ensemble::fetcher::Fetcher<CTXEXT>
            + Send
            + Sync
            + 'static,
        >,
    >,
) -> Result<super::VectorCompletionFlatTaskProfile, super::executions::Error>
where
    CTXEXT: Send + Sync + 'static,
{
    // switch based on profile
    let ensemble = match ensemble {
        objectiveai::vector::completions::request::Ensemble::Id(id) => {
            // fetch ensemble
            ensemble_fetcher
                .fetch(ctx, &id)
                .map(|result| match result {
                    Ok(Some((ensemble, _))) => Ok(ensemble),
                    Ok(None) => Err(super::executions::Error::EnsembleNotFound),
                    Err(e) => Err(super::executions::Error::FetchEnsemble(e)),
                })
                .await?
        }
        objectiveai::vector::completions::request::Ensemble::Provided(
            ensemble,
        ) => {
            // validate ensemble
            ensemble
                .clone()
                .try_into()
                .map_err(super::executions::Error::InvalidEnsemble)?
        }
    };

    // validate profile length
    if profile.len() != ensemble.llms.len() {
        return Err(super::executions::Error::InvalidProfile);
    }

    // construct flat task profile
    Ok(super::VectorCompletionFlatTaskProfile {
        path,
        ensemble: objectiveai::ensemble::EnsembleBase {
            llms: ensemble
                .llms
                .into_iter()
                .map(|llm| {
                    objectiveai::ensemble_llm::EnsembleLlmBaseWithFallbacksAndCount {
                        count: llm.count,
                        inner: llm.inner.base,
                        fallbacks: llm.fallbacks.map(|fallbacks| {
                            fallbacks
                                .into_iter()
                                .map(|fallback| fallback.base)
                                .collect()
                        }),
                    }
                })
                .collect(),
        },
        profile,
        messages: task.messages,
        tools: task.tools,
        responses: task.responses,
    })
}

enum TaskFut<
    VFUT: Future<
        Output = Result<
            super::VectorCompletionFlatTaskProfile,
            super::executions::Error,
        >,
    >,
    FFUT: Future<
        Output = Result<
            super::FunctionFlatTaskProfile,
            super::executions::Error,
        >,
    >,
> {
    SkipTask,
    Task(Option<super::FlatTaskProfile>),
    VectorTaskFut(Pin<Box<VFUT>>),
    MapVectorTaskFut((Vec<u64>, futures::future::TryJoinAll<VFUT>)),
    FunctionTaskFut(Pin<Box<FFUT>>),
    MapFunctionTaskFut((Vec<u64>, futures::future::TryJoinAll<FFUT>)),
}

impl<VFUT, FFUT> Future for TaskFut<VFUT, FFUT>
where
    VFUT: Future<
        Output = Result<
            super::VectorCompletionFlatTaskProfile,
            super::executions::Error,
        >,
    >,
    FFUT: Future<
        Output = Result<
            super::FunctionFlatTaskProfile,
            super::executions::Error,
        >,
    >,
{
    type Output =
        Result<Option<super::FlatTaskProfile>, super::executions::Error>;
    fn poll(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Self::Output> {
        match self.get_mut() {
            TaskFut::SkipTask => Poll::Ready(Ok(None)),
            TaskFut::Task(task) => Poll::Ready(Ok(task.take())),
            TaskFut::VectorTaskFut(fut) => Pin::new(fut)
                .poll(cx)
                .map_ok(FlatTaskProfile::VectorCompletion)
                .map_ok(Some),
            TaskFut::MapVectorTaskFut((path, futs)) => {
                Pin::new(futs).poll(cx).map_ok(|results| {
                    Some(FlatTaskProfile::MapVectorCompletion(
                        MapVectorCompletionFlatTaskProfile {
                            path: path.clone(),
                            vector_completions: results,
                        },
                    ))
                })
            }
            TaskFut::FunctionTaskFut(fut) => Pin::new(fut)
                .poll(cx)
                .map_ok(FlatTaskProfile::Function)
                .map_ok(Some),
            TaskFut::MapFunctionTaskFut((path, futs)) => {
                Pin::new(futs).poll(cx).map_ok(|results| {
                    Some(FlatTaskProfile::MapFunction(
                        MapFunctionFlatTaskProfile {
                            path: path.clone(),
                            functions: results,
                        },
                    ))
                })
            }
        }
    }
}
