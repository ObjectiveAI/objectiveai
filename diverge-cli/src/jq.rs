//! Running a jq program over a JSON value: what a `<program> config
//! get` narrows by, a `<program> config set` rewrites with, and a `run --jq` prints
//! through. Querying and rewriting whole; a change to one place is a
//! patch, never jq.

use std::fmt;
use std::iter::Empty;

use jaq_core::{Compiler, Ctx, Filter, Native, RcIter, load};
use jaq_json::Val;
use serde_json::Value;

/// A jq program, compiled once and run over any number of values.
pub struct Program {
    /// The compiled filter.
    filter: Filter<Native<Val>>,
    /// The program's input stream, which a run borrows: empty, the
    /// input of a run being the value it is given.
    inputs: RcIter<Empty<Result<Val, String>>>,
}

impl Program {
    /// `code` parsed and compiled with jaq's and jq's standard
    /// definitions; a program that does not parse, or does not compile
    /// — a name it defines nowhere — is the [`Error`], in jaq's own
    /// words.
    pub fn new(code: &str) -> Result<Program, Error> {
        let program = load::File { code, path: () };
        let loader = load::Loader::new(jaq_std::defs().chain(jaq_json::defs()));
        let arena = load::Arena::default();
        let modules = loader.load(&arena, program).map_err(|errors| {
            Error(
                errors
                    .into_iter()
                    .map(|error| format!("{error:?}"))
                    .collect::<Vec<_>>()
                    .join(", "),
            )
        })?;
        let filter = Compiler::default()
            .with_funs(jaq_std::funs().chain(jaq_json::funs()))
            .compile(modules)
            .map_err(|errors| {
                Error(
                    errors
                        .into_iter()
                        .map(|error| format!("{error:?}"))
                        .collect::<Vec<_>>()
                        .join(", "),
                )
            })?;
        Ok(Program {
            filter,
            inputs: RcIter::new(core::iter::empty()),
        })
    }

    /// Every value the program yields over `input`, in order, as the
    /// program yields it: each is taken as it is computed, so the first
    /// is had before the last is. A failure as the program runs is the
    /// [`Error`], in jaq's own words, after the values yielded before
    /// it.
    pub fn run<'a>(&'a self, input: Value) -> impl Iterator<Item = Result<Value, Error>> + 'a {
        self.filter
            .run((Ctx::new(Vec::new(), &self.inputs), Val::from(input)))
            .map(|item| match item {
                Ok(value) => Ok(Value::from(value)),
                Err(error) => Err(Error(format!("{error:?}"))),
            })
    }
}

/// Every value `filter` yields over `input`, in order, collected. A
/// program that does not parse, does not compile, or fails as it runs
/// is the [`Error`], in jaq's own words; values it yielded before a
/// failure are not returned. For a program run once, over one value; a
/// program run over many is a [`Program`].
pub fn run(filter: &str, input: Value) -> Result<Vec<Value>, Error> {
    Program::new(filter)?.run(input).collect()
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
