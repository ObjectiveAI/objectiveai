//! Recursive non-collecting iterators that yield every
//! `agent_instance_hierarchy` referenced inside a chunk.
//!
//! Mirrors the walker pattern in [`crate::db::logs::rows`]: a free
//! function per chunk type, each returning a boxed iterator of borrowed
//! `&str` slices into the chunk. The [`ChunkAgentHierarchies`] trait
//! makes the dispatch usable from generic code (e.g.
//! `streaming::run_chunk_loop`).

use objectiveai_sdk::agent::completions::response::streaming::AgentCompletionChunk;
use objectiveai_sdk::functions::executions::response::streaming::{
    FunctionExecutionChunk, TaskChunk,
};
use objectiveai_sdk::vector::completions::response::streaming::VectorCompletionChunk;

pub type HierIter<'a> = Box<dyn Iterator<Item = &'a str> + Send + 'a>;

pub fn agent_completion_chunk_hierarchies<'a>(
    chunk: &'a AgentCompletionChunk,
) -> HierIter<'a> {
    Box::new(std::iter::once(chunk.agent_instance_hierarchy.as_str()))
}

pub fn vector_completion_chunk_hierarchies<'a>(
    chunk: &'a VectorCompletionChunk,
) -> HierIter<'a> {
    Box::new(
        chunk
            .completions
            .iter()
            .flat_map(|c| agent_completion_chunk_hierarchies(&c.inner)),
    )
}

pub fn function_execution_chunk_hierarchies<'a>(
    chunk: &'a FunctionExecutionChunk,
) -> HierIter<'a> {
    let from_tasks = chunk.tasks.iter().flat_map(|t| task_chunk_hierarchies(t));
    let from_reasoning = chunk
        .reasoning
        .iter()
        .flat_map(|r| agent_completion_chunk_hierarchies(&r.inner));
    Box::new(from_tasks.chain(from_reasoning))
}

fn task_chunk_hierarchies<'a>(task: &'a TaskChunk) -> HierIter<'a> {
    match task {
        TaskChunk::FunctionExecution(w) => {
            function_execution_chunk_hierarchies(&w.inner)
        }
        TaskChunk::VectorCompletion(w) => {
            vector_completion_chunk_hierarchies(&w.inner)
        }
    }
}

pub trait ChunkAgentHierarchies {
    fn agent_instance_hierarchies<'a>(&'a self) -> HierIter<'a>;
}

impl ChunkAgentHierarchies for AgentCompletionChunk {
    fn agent_instance_hierarchies<'a>(&'a self) -> HierIter<'a> {
        agent_completion_chunk_hierarchies(self)
    }
}

impl ChunkAgentHierarchies for VectorCompletionChunk {
    fn agent_instance_hierarchies<'a>(&'a self) -> HierIter<'a> {
        vector_completion_chunk_hierarchies(self)
    }
}

impl ChunkAgentHierarchies for FunctionExecutionChunk {
    fn agent_instance_hierarchies<'a>(&'a self) -> HierIter<'a> {
        function_execution_chunk_hierarchies(self)
    }
}
