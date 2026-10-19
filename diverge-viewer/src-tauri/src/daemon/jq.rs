//! A logs request's `jq`, run the way the daemon runs one — the same crates
//! and the same shape as `objectiveai-daemon/src/filesystem/jq.rs`.

use jaq_core::{Compiler, Ctx, RcIter, load};
use jaq_json::Val;

/// Run `filter` over `value`; everything it yields, in order.
pub fn run(value: serde_json::Value, filter: &str) -> Result<Vec<serde_json::Value>, String> {
    let program = load::File { code: filter, path: () };
    let loader = load::Loader::new(jaq_std::defs().chain(jaq_json::defs()));
    let arena = load::Arena::default();
    let modules = loader
        .load(&arena, program)
        .map_err(|errs| format!("jq did not parse: {}", errs.into_iter().map(|e| format!("{e:?}")).collect::<Vec<_>>().join(", ")))?;
    let filter = Compiler::default()
        .with_funs(jaq_std::funs().chain(jaq_json::funs()))
        .compile(modules)
        .map_err(|errs| format!("jq did not compile: {}", errs.into_iter().map(|e| format!("{e:?}")).collect::<Vec<_>>().join(", ")))?;
    let inputs = RcIter::new(core::iter::empty());
    let out = filter.run((Ctx::new([], &inputs), Val::from(value)));
    let mut results = Vec::new();
    for item in out {
        match item {
            Ok(val) => results.push(serde_json::Value::from(val)),
            Err(err) => return Err(format!("jq failed: {err:?}")),
        }
    }
    Ok(results)
}
