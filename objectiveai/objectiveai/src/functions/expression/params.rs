use crate::vector;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum Params<'i, 't, 'to, 'm> {
    Owned(ParamsOwned),
    Ref(ParamsRef<'i, 't, 'to, 'm>),
}

impl<'de> serde::Deserialize<'de>
    for Params<'static, 'static, 'static, 'static>
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let owned = ParamsOwned::deserialize(deserializer)?;
        Ok(Params::Owned(owned))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamsOwned {
    pub input: super::Input,
    pub tasks: Vec<Option<TaskOutputOwned>>, // only provided to output expressions
    pub map: Option<super::Input>, // only provided to task expressions, other than skip
}

#[derive(Debug, Clone, Serialize)]
pub struct ParamsRef<'i, 't, 'to, 'm> {
    pub input: &'i super::Input,
    pub tasks: &'t [Option<TaskOutput<'to>>], // only provided to output expressions
    pub map: Option<&'m super::Input>, // only provided to task expressions, other than skip
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum TaskOutput<'a> {
    Owned(TaskOutputOwned),
    Ref(TaskOutputRef<'a>),
}

impl<'de> serde::Deserialize<'de> for TaskOutput<'static> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let owned = TaskOutputOwned::deserialize(deserializer)?;
        Ok(TaskOutput::Owned(owned))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TaskOutputOwned {
    Function(FunctionOutput),
    MapFunction(Vec<FunctionOutput>),
    VectorCompletion(VectorCompletionOutput),
    MapVectorCompletion(Vec<VectorCompletionOutput>),
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum TaskOutputRef<'a> {
    Function(&'a FunctionOutput),
    MapFunction(&'a [FunctionOutput]),
    VectorCompletion(&'a VectorCompletionOutput),
    MapVectorCompletion(&'a [VectorCompletionOutput]),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorCompletionOutput {
    pub votes: Vec<vector::completions::response::Vote>,
    pub scores: Vec<rust_decimal::Decimal>,
    pub weights: Vec<rust_decimal::Decimal>,
}

impl From<vector::completions::response::streaming::VectorCompletionChunk>
    for VectorCompletionOutput
{
    fn from(
        vector::completions::response::streaming::VectorCompletionChunk {
            votes,
            scores,
            weights,
            ..
        }: vector::completions::response::streaming::VectorCompletionChunk,
    ) -> Self {
        VectorCompletionOutput {
            votes,
            scores,
            weights,
        }
    }
}

impl From<vector::completions::response::unary::VectorCompletion>
    for VectorCompletionOutput
{
    fn from(
        vector::completions::response::unary::VectorCompletion {
            votes,
            scores,
            weights,
            ..
        }: vector::completions::response::unary::VectorCompletion,
    ) -> Self {
        VectorCompletionOutput {
            votes,
            scores,
            weights,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FunctionOutput {
    Scalar(rust_decimal::Decimal),
    Vector(Vec<rust_decimal::Decimal>),
    Err(serde_json::Value),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledFunctionOutput {
    pub output: FunctionOutput,
    pub valid: bool,
}
