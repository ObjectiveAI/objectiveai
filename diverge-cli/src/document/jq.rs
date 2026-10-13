//! Running a jq program over a document.

use jaq_core::{Compiler, Ctx, RcIter, load};
use jaq_json::Val;
use serde_json::Value;

use super::Error;

/// Every value `filter` yields over `input`, in order. A program that
/// does not parse, does not compile — a name it defines nowhere — or
/// fails as it runs is [`Error::Jq`], in jaq's own words; values it
/// yielded before a failure are not returned.
pub fn run(filter: &str, input: Value) -> Result<Vec<Value>, Error> {
    let program = load::File { code: filter, path: () };
    let loader = load::Loader::new(jaq_std::defs().chain(jaq_json::defs()));
    let arena = load::Arena::default();
    let modules = loader
        .load(&arena, program)
        .map_err(|errors| Error::Jq(errors.into_iter().map(|error| format!("{error:?}")).collect::<Vec<_>>().join(", ")))?;
    let compiled = Compiler::default()
        .with_funs(jaq_std::funs().chain(jaq_json::funs()))
        .compile(modules)
        .map_err(|errors| Error::Jq(errors.into_iter().map(|error| format!("{error:?}")).collect::<Vec<_>>().join(", ")))?;
    let inputs = RcIter::new(core::iter::empty());
    let mut values = Vec::new();
    for item in compiled.run((Ctx::new([], &inputs), Val::from(input))) {
        match item {
            Ok(value) => values.push(Value::from(value)),
            Err(error) => return Err(Error::Jq(format!("{error:?}"))),
        }
    }
    Ok(values)
}
