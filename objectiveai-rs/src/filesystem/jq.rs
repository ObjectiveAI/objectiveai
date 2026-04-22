use jaq_core::{load, Compiler, Ctx, FilterT, RcIter};
use jaq_json::Val;
use serde::Serialize;

pub fn run_jq(value: &impl Serialize, filter: &str) -> Result<Vec<serde_json::Value>, super::Error> {
    let program = load::File { code: filter, path: () };
    let loader = load::Loader::new(jaq_std::defs().chain(jaq_json::defs()));
    let arena = load::Arena::default();

    let modules = loader.load(&arena, program)
        .map_err(|errs| super::Error::JqParse(
            errs.into_iter().map(|e| format!("{e:?}")).collect::<Vec<_>>().join(", ")
        ))?;

    let filter = Compiler::default()
        .with_funs(jaq_std::funs().chain(jaq_json::funs()))
        .compile(modules)
        .map_err(|errs| super::Error::JqCompile(
            errs.into_iter().map(|e| format!("{e:?}")).collect::<Vec<_>>().join(", ")
        ))?;

    let input_value = serde_json::to_value(value)
        .map_err(super::Error::Serialize)?;

    let inputs = RcIter::new(core::iter::empty());
    let out = filter.run((Ctx::new([], &inputs), Val::from(input_value)));

    let mut results = Vec::new();
    for item in out {
        match item {
            Ok(val) => results.push(serde_json::Value::from(val)),
            Err(err) => return Err(super::Error::JqRuntime(format!("{err:?}"))),
        }
    }
    Ok(results)
}
