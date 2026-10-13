//! Running a jq program over a JSON value: what a `config get`
//! narrows by, a `config set` and `config edit` rewrite with, and a
//! `run --jq` prints through.

use std::fmt;

use jaq_core::{Compiler, Ctx, RcIter, load};
use jaq_json::Val;
use serde_json::Value;

/// Every value `filter` yields over `input`, in order. With `value`,
/// the program may name `$value`, which is bound to it; without,
/// naming it is an error. A program that does not parse, does not
/// compile — a name it defines nowhere — or fails as it runs is
/// the [`Error`], in jaq's own words; values it yielded before a
/// failure are not returned.
pub fn run(filter: &str, input: Value, value: Option<Value>) -> Result<Vec<Value>, Error> {
    let program = load::File { code: filter, path: () };
    let loader = load::Loader::new(jaq_std::defs().chain(jaq_json::defs()));
    let arena = load::Arena::default();
    let modules = loader
        .load(&arena, program)
        .map_err(|errors| Error(errors.into_iter().map(|error| format!("{error:?}")).collect::<Vec<_>>().join(", ")))?;
    let globals: Vec<&str> = value.iter().map(|_| "$value").collect();
    let compiled = Compiler::default()
        .with_funs(jaq_std::funs().chain(jaq_json::funs()))
        .with_global_vars(globals)
        .compile(modules)
        .map_err(|errors| Error(errors.into_iter().map(|error| format!("{error:?}")).collect::<Vec<_>>().join(", ")))?;
    let inputs = RcIter::new(core::iter::empty());
    let mut values = Vec::new();
    let vars: Vec<Val> = value.into_iter().map(Val::from).collect();
    for item in compiled.run((Ctx::new(vars, &inputs), Val::from(input))) {
        match item {
            Ok(value) => values.push(Value::from(value)),
            Err(error) => return Err(Error(format!("{error:?}"))),
        }
    }
    Ok(values)
}

/// Why a jq program could not be run: it did not parse, did not
/// compile, or failed as it ran, in jaq's own words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the jq program: {}", self.0)
    }
}

impl std::error::Error for Error {}
