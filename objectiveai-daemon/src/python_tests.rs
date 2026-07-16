//! Contract tests for the WASI python harness: bare-trailing-
//! expression detection, the eval/stdout envelope, and error
//! classification. The harness deliberately has NO safeguards against
//! user code breaking envelope emission, so there are no
//! monkey-patch-defense tests — only contract behavior.
//!
//! The runtime is shared across tests via a `OnceCell` and caches its
//! JIT artifact under the OS temp dir (one compile per machine, never
//! the user's real layout root).

use crate::context::{GlobalContext, ScopedContext};
use crate::python::Python;

#[derive(Debug, PartialEq, serde::Deserialize)]
struct Foo {
    foo: String,
}

fn expected() -> Foo {
    Foo {
        foo: "bar".to_string(),
    }
}

async fn py() -> &'static Python {
    static CELL: tokio::sync::OnceCell<Python> = tokio::sync::OnceCell::const_new();
    CELL.get_or_init(|| async {
        let bin_dir = std::env::temp_dir().join("objectiveai-python-tests");
        Python::initialize(bin_dir)
            .await
            .expect("initialize WASI python runtime")
    })
    .await
}

/// `Ok(None)` ⇒ the script produced no usable output (no trailing
/// expression and nothing printed); `Ok(Some(v))` ⇒ a value (including
/// `Some(serde_json::Value::Null)` when the script explicitly emits JSON
/// `null` on stdout). Callers that require a value `.unwrap()` the option.
/// A throwaway context pair for the context-aware `exec_code`/`exec_file`
/// API. These harness tests never call `objectiveai.execute`, so the pair is
/// only along for the signature — defaults (no real layout root) are fine.
fn ctx() -> (crate::context::GlobalContext, crate::context::ScopedContext) {
    let config = crate::run::ConfigBuilder::default().build();
    (
        crate::context::GlobalContext::new(&config),
        crate::context::ScopedContext::boot(&config),
    )
}

async fn exec<T: serde::de::DeserializeOwned>(
    code: &str,
) -> Result<Option<T>, crate::error::Error> {
    let (global, scoped) = ctx();
    py().await.exec_code::<(), T>(&global, &scoped, code, None).await
}

// -- Bare expressions and prints --

/// Bare dict literal, starlark-style — no print, just an expression.
#[tokio::test]
async fn eval_dict_literal() {
    let result: Foo = exec(r#"{"foo": "bar"}"#).await.unwrap().unwrap();
    assert_eq!(result, expected());
}

/// Explicit print with json.dumps on a single line.
#[tokio::test]
async fn print_dict() {
    let result: Foo = exec(r#"import json; print(json.dumps({"foo": "bar"}))"#)
        .await
        .unwrap().unwrap();
    assert_eq!(result, expected());
}

/// Defines main() that returns the dict, calls it inside if __name__ == "__main__".
#[tokio::test]
async fn main_returns() {
    let result: Foo = exec(
        r#"
import json

def main():
    return {"foo": "bar"}

if __name__ == "__main__":
    print(json.dumps(main()))
"#,
    )
    .await
    .unwrap().unwrap();
    assert_eq!(result, expected());
}

/// Dict printed inside if __name__ == "__main__" guard, starlark-style.
#[tokio::test]
async fn starlark_style_in_main_guard() {
    let result: Foo = exec(
        r#"
import json

if __name__ == "__main__":
    print(json.dumps({"foo": "bar"}))
"#,
    )
    .await
    .unwrap().unwrap();
    assert_eq!(result, expected());
}

/// Defines main(), prints its return value at top level.
#[tokio::test]
async fn print_main_return() {
    let result: Foo = exec(
        r#"
import json

def main():
    return {"foo": "bar"}

print(json.dumps(main()))
"#,
    )
    .await
    .unwrap().unwrap();
    assert_eq!(result, expected());
}

/// Unused function definition followed by a print.
#[tokio::test]
async fn unused_fn_then_print() {
    let result: Foo = exec(
        r#"
import json

def add(a, b):
    return a + b

print(json.dumps({"foo": "bar"}))
"#,
    )
    .await
    .unwrap().unwrap();
    assert_eq!(result, expected());
}

/// Uses sys.stdout.write instead of print — no trailing newline.
#[tokio::test]
async fn sys_stdout_write() {
    let result: Foo = exec(
        r#"
import json, sys
sys.stdout.write(json.dumps({"foo": "bar"}))
"#,
    )
    .await
    .unwrap().unwrap();
    assert_eq!(result, expected());
}

/// Defines a function and calls it as a bare expression (no print).
#[tokio::test]
async fn bare_function_call() {
    let result: Foo = exec(
        r#"
def get():
    return {"foo": "bar"}

get()
"#,
    )
    .await
    .unwrap().unwrap();
    assert_eq!(result, expected());
}

/// Assigns to a variable, then references it as a bare expression.
#[tokio::test]
async fn assign_then_bare_variable() {
    let result: Foo = exec(
        r#"
x = {"foo": "bar"}
x
"#,
    )
    .await
    .unwrap().unwrap();
    assert_eq!(result, expected());
}

/// Dict literal split across multiple lines with trailing comma.
#[tokio::test]
async fn multiline_dict_expression() {
    let result: Foo = exec(
        r#"
{
    "foo": "bar",
}
"#,
    )
    .await
    .unwrap().unwrap();
    assert_eq!(result, expected());
}

/// Dict comprehension as a bare expression.
#[tokio::test]
async fn dict_comprehension() {
    let result: Foo = exec(r#"{k: v for k, v in [("foo", "bar")]}"#).await.unwrap().unwrap();
    assert_eq!(result, expected());
}

/// Ternary/conditional expression returning a dict.
#[tokio::test]
async fn ternary_expression() {
    let result: Foo = exec(r#"{"foo": "bar"} if True else None"#).await.unwrap().unwrap();
    assert_eq!(result, expected());
}

/// Walrus operator (:=) as a bare expression.
#[tokio::test]
async fn walrus_operator() {
    let result: Foo = exec(r#"(x := {"foo": "bar"})"#).await.unwrap().unwrap();
    assert_eq!(result, expected());
}

/// Immediately-invoked lambda returning a dict.
#[tokio::test]
async fn lambda_call() {
    let result: Foo = exec(r#"(lambda: {"foo": "bar"})()"#).await.unwrap().unwrap();
    assert_eq!(result, expected());
}

/// Two statements on one line separated by semicolon, last is a bare expression.
#[tokio::test]
async fn semicolons_one_line() {
    let result: Foo = exec(r#"x = 1; {"foo": "bar"}"#).await.unwrap().unwrap();
    assert_eq!(result, expected());
}

/// Uses dict() constructor instead of literal syntax.
#[tokio::test]
async fn nested_function_call_dict() {
    let result: Foo = exec(r#"dict(foo="bar")"#).await.unwrap().unwrap();
    assert_eq!(result, expected());
}

/// Prints debug info to stderr, then has a bare expression on the last line.
#[tokio::test]
async fn stderr_debug_then_bare_expression() {
    let result: Foo = exec(
        r#"
import sys
print("debug info", file=sys.stderr)
{"foo": "bar"}
"#,
    )
    .await
    .unwrap().unwrap();
    assert_eq!(result, expected());
}

/// Prints unrelated text to stdout, then has a bare expression as the last line.
/// The wrapper should use the eval'd expression, not the printed noise.
#[tokio::test]
async fn stdout_noise_then_bare_expression() {
    let result: Foo = exec(
        r#"
print("some random debug output")
{"foo": "bar"}
"#,
    )
    .await
    .unwrap().unwrap();
    assert_eq!(result, expected());
}

/// Defines a class with a method, calls it as a bare expression.
#[tokio::test]
async fn class_method_call() {
    let result: Foo = exec(
        r#"
class C:
    def get(self):
        return {"foo": "bar"}

C().get()
"#,
    )
    .await
    .unwrap().unwrap();
    assert_eq!(result, expected());
}

/// Prints JSON with end="" (no trailing newline).
#[tokio::test]
async fn print_no_newline() {
    let result: Foo = exec(r#"import json; print(json.dumps({"foo": "bar"}), end="")"#)
        .await
        .unwrap().unwrap();
    assert_eq!(result, expected());
}

/// Multiple bare expressions — only the last one is captured by the harness.
#[tokio::test]
async fn multiple_bare_expressions_last_wins() {
    let result: Foo = exec(
        r#"
1
2
{"foo": "bar"}
"#,
    )
    .await
    .unwrap().unwrap();
    assert_eq!(result, expected());
}

/// Bare expression followed by trailing blank lines.
#[tokio::test]
async fn trailing_blank_lines() {
    let result: Foo = exec("{\"foo\": \"bar\"}\n\n\n").await.unwrap().unwrap();
    assert_eq!(result, expected());
}

/// Bare expression followed by a trailing comment.
#[tokio::test]
async fn trailing_comment_after_expression() {
    let result: Foo = exec("{\"foo\": \"bar\"}\n# done").await.unwrap().unwrap();
    assert_eq!(result, expected());
}

// -- User-code shapes --

/// Expression followed by a trailing `pass` statement — expression is no longer last,
/// so the user must print explicitly.
#[tokio::test]
async fn trailing_pass_after_expression() {
    let result: Foo = exec(
        r#"
import json
print(json.dumps({"foo": "bar"}))
pass
"#,
    )
    .await
    .unwrap().unwrap();
    assert_eq!(result, expected());
}

/// User code defines underscore-prefixed variables of its own.
#[tokio::test]
async fn user_defines_underscore_variables() {
    let result: Foo = exec(
        r#"
import json
_json = None
_capture = "oops"
print(json.dumps({"foo": "bar"}))
"#,
    )
    .await
    .unwrap().unwrap();
    assert_eq!(result, expected());
}

/// User code runs in its own globals: `__oai_*` names set by the user
/// don't collide with the harness scope.
#[tokio::test]
async fn user_sets_oai_prefix_variable() {
    let result: Foo = exec(
        r#"
import json
__oai_capture = "sabotage"
__oai_json = None
print(json.dumps({"foo": "bar"}))
"#,
    )
    .await
    .unwrap().unwrap();
    assert_eq!(result, expected());
}

/// User code deletes its own `sys` binding — the harness scope is separate.
#[tokio::test]
async fn user_deletes_sys() {
    let result: Foo = exec(
        r#"
import json, sys
del sys
print(json.dumps({"foo": "bar"}))
"#,
    )
    .await
    .unwrap().unwrap();
    assert_eq!(result, expected());
}

/// User code rebinds `print` in its own globals; the builtin stays
/// reachable via `builtins`.
#[tokio::test]
async fn redefine_print_use_builtins() {
    let result: Foo = exec(
        r#"
import json, builtins
print = lambda *a: None
builtins.print(json.dumps({"foo": "bar"}))
"#,
    )
    .await
    .unwrap().unwrap();
    assert_eq!(result, expected());
}

/// Variable defined on one line, used in a bare expression on the next.
#[tokio::test]
async fn global_variable_in_expression() {
    let result: Foo = exec(
        r#"
x = "bar"
{"foo": x}
"#,
    )
    .await
    .unwrap().unwrap();
    assert_eq!(result, expected());
}

/// User code itself calls exec() to define a variable, then prints it.
#[tokio::test]
async fn nested_exec_in_user_code() {
    let result: Foo = exec(
        r#"
import json
exec('result = {"foo": "bar"}')
print(json.dumps(result))
"#,
    )
    .await
    .unwrap().unwrap();
    assert_eq!(result, expected());
}

/// Code that raises an exception, catches it, then returns via bare expression.
#[tokio::test]
async fn try_except_then_bare_expression() {
    let result: Foo = exec(
        r#"
try:
    raise ValueError("oops")
except:
    pass
{"foo": "bar"}
"#,
    )
    .await
    .unwrap().unwrap();
    assert_eq!(result, expected());
}

/// Code that uses __name__ inside an if guard — verifies __name__ == "__main__" in exec().
#[tokio::test]
async fn name_equals_main_in_exec() {
    let result: Foo = exec(
        r#"
import json
if __name__ == "__main__":
    print(json.dumps({"foo": "bar"}))
"#,
    )
    .await
    .unwrap().unwrap();
    assert_eq!(result, expected());
}

/// Bare expression after a for loop.
#[tokio::test]
async fn bare_expression_after_for_loop() {
    let result: Foo = exec(
        r#"
items = []
for i in range(3):
    items.append(i)
{"foo": "bar"}
"#,
    )
    .await
    .unwrap().unwrap();
    assert_eq!(result, expected());
}

/// Bare expression after a with statement.
#[tokio::test]
async fn bare_expression_after_with() {
    let result: Foo = exec(
        r#"
import io
with io.StringIO() as f:
    f.write("ignored")
{"foo": "bar"}
"#,
    )
    .await
    .unwrap().unwrap();
    assert_eq!(result, expected());
}

/// Bare expression after try/finally.
#[tokio::test]
async fn bare_expression_after_try_finally() {
    let result: Foo = exec(
        r#"
x = None
try:
    x = 1
finally:
    x = 2
{"foo": "bar"}
"#,
    )
    .await
    .unwrap().unwrap();
    assert_eq!(result, expected());
}

/// Code that uses globals() to set a variable, then references it in bare expression.
#[tokio::test]
async fn globals_dict_then_bare_expression() {
    let result: Foo = exec(
        r#"
globals()["x"] = "bar"
{"foo": x}
"#,
    )
    .await
    .unwrap().unwrap();
    assert_eq!(result, expected());
}

/// Code that uses __import__ for dynamic import.
#[tokio::test]
async fn dynamic_import() {
    let result: Foo = exec(
        r#"
json = __import__("json")
print(json.dumps({"foo": "bar"}))
"#,
    )
    .await
    .unwrap().unwrap();
    assert_eq!(result, expected());
}

/// Deeply nested dict as bare expression.
#[tokio::test]
async fn deeply_nested_dict() {
    let result: serde_json::Value = exec(
        r#"
{"a": {"b": {"c": {"foo": "bar"}}}}
"#,
    )
    .await
    .unwrap().unwrap();
    assert_eq!(result["a"]["b"]["c"]["foo"], "bar");
}

/// Multiline string containing what looks like Python code, followed by bare expression.
#[tokio::test]
async fn multiline_string_then_bare_expression() {
    let result: Foo = exec(
        r#"
code = """
def fake():
    return {"wrong": "value"}
print("this is not executed")
"""
{"foo": "bar"}
"#,
    )
    .await
    .unwrap().unwrap();
    assert_eq!(result, expected());
}

/// Unicode and emoji in the dict value.
#[tokio::test]
async fn unicode_emoji_value() {
    #[derive(Debug, PartialEq, serde::Deserialize)]
    struct Uni {
        msg: String,
    }
    let result: Uni = exec(
        r#"
{"msg": "hello 🦀 world àéîõü"}
"#,
    )
    .await
    .unwrap().unwrap();
    assert_eq!(
        result,
        Uni {
            msg: "hello 🦀 world àéîõü".to_string()
        }
    );
}

/// Code that uses *args and **kwargs, then bare expression.
#[tokio::test]
async fn args_kwargs_then_bare_expression() {
    let result: Foo = exec(
        r#"
def make(*args, **kwargs):
    return kwargs

make("ignored", foo="bar")
"#,
    )
    .await
    .unwrap().unwrap();
    assert_eq!(result, expected());
}

/// Code that uses type() to dynamically create a class, then calls a method.
#[tokio::test]
async fn dynamic_class_creation() {
    let result: Foo = exec(
        r#"
MyClass = type("MyClass", (), {"get": lambda self: {"foo": "bar"}})
MyClass().get()
"#,
    )
    .await
    .unwrap().unwrap();
    assert_eq!(result, expected());
}

/// List comprehension producing a single-element list, indexed to get the dict.
#[tokio::test]
async fn list_comprehension_indexed() {
    let result: Foo = exec(
        r#"
[{"foo": v} for v in ["bar"]][0]
"#,
    )
    .await
    .unwrap().unwrap();
    assert_eq!(result, expected());
}

/// Code with an `exit()` guard that is never taken.
#[tokio::test]
async fn conditional_with_no_exit() {
    let result: Foo = exec(
        r#"
import json
should_exit = False
if should_exit:
    exit(1)
print(json.dumps({"foo": "bar"}))
"#,
    )
    .await
    .unwrap().unwrap();
    assert_eq!(result, expected());
}

/// Garbage printed before the correct JSON: captured stdout is the
/// concatenation, which doesn't deserialize as Foo.
#[tokio::test]
async fn garbage_stdout_before_correct_print() {
    let err = exec::<Foo>(
        r#"
import json
print("here is some random garbage!!!")
print(json.dumps({"foo": "bar"}))
"#,
    )
    .await
    .unwrap_err();
    assert!(matches!(err, crate::error::Error::PythonDeserialize(_)));
}

// -- PythonException errors --

/// Bare `def` is a syntax error.
#[tokio::test]
async fn error_syntax_error() {
    let err = exec::<Foo>("def").await.unwrap_err();
    assert!(matches!(err, crate::error::Error::PythonException(_)));
}

/// Reference to an undefined variable.
#[tokio::test]
async fn error_name_error() {
    let err = exec::<Foo>("undefined_variable").await.unwrap_err();
    assert!(matches!(err, crate::error::Error::PythonException(_)));
}

/// Division by zero.
#[tokio::test]
async fn error_zero_division() {
    let err = exec::<Foo>("1 / 0").await.unwrap_err();
    assert!(matches!(err, crate::error::Error::PythonException(_)));
}

/// Import a module that doesn't exist.
#[tokio::test]
async fn error_import_error() {
    let err = exec::<Foo>("import nonexistent_module_xyz").await.unwrap_err();
    assert!(matches!(err, crate::error::Error::PythonException(_)));
}

/// Infinite recursion: either the interpreter's own RecursionError
/// (PythonException) or, if the wasm stack gives out first, a trap
/// (PythonWasm). Both are acceptable failures; what matters is that
/// the host survives and classifies it as an error.
#[tokio::test]
async fn error_recursion_error() {
    let err = exec::<Foo>(
        r#"
def f(): f()
f()
"#,
    )
    .await
    .unwrap_err();
    assert!(matches!(
        err,
        crate::error::Error::PythonException(_) | crate::error::Error::PythonWasm(_)
    ));
}

/// Explicit raise.
#[tokio::test]
async fn error_explicit_raise() {
    let err = exec::<Foo>(r#"raise RuntimeError("boom")"#).await.unwrap_err();
    assert!(matches!(err, crate::error::Error::PythonException(_)));
}

/// sys.exit(1) exits the interpreter with a nonzero status before the
/// envelope is printed — a PythonException.
#[tokio::test]
async fn error_sys_exit_nonzero() {
    let err = exec::<Foo>("import sys; sys.exit(1)").await.unwrap_err();
    assert!(matches!(err, crate::error::Error::PythonException(_)));
}

/// sys.exit(0) exits CLEANLY before the envelope is printed. With no
/// harness safeguards, that reads as a malformed envelope.
#[tokio::test]
async fn error_sys_exit_zero_breaks_envelope() {
    let err = exec::<Foo>("import sys; sys.exit(0)").await.unwrap_err();
    assert!(matches!(err, crate::error::Error::PythonHarnessBroken(_)));
}

/// TypeError from wrong operation.
#[tokio::test]
async fn error_type_error() {
    let err = exec::<Foo>(r#""hello" + 5"#).await.unwrap_err();
    assert!(matches!(err, crate::error::Error::PythonException(_)));
}

/// Exception in the eval'd expression (exec part succeeds, eval fails).
#[tokio::test]
async fn error_exception_in_eval_expression() {
    let err = exec::<Foo>(
        r#"
x = 1
1 / 0
"#,
    )
    .await
    .unwrap_err();
    assert!(matches!(err, crate::error::Error::PythonException(_)));
}

/// Assertion error.
#[tokio::test]
async fn error_assert_false() {
    let err = exec::<Foo>("assert False, 'nope'").await.unwrap_err();
    assert!(matches!(err, crate::error::Error::PythonException(_)));
}

/// KeyError from dict access.
#[tokio::test]
async fn error_key_error() {
    let err = exec::<Foo>("{}['missing']").await.unwrap_err();
    assert!(matches!(err, crate::error::Error::PythonException(_)));
}

/// IndexError from list access.
#[tokio::test]
async fn error_index_error() {
    let err = exec::<Foo>("[][0]").await.unwrap_err();
    assert!(matches!(err, crate::error::Error::PythonException(_)));
}

/// AttributeError from accessing nonexistent attribute.
#[tokio::test]
async fn error_attribute_error() {
    let err = exec::<Foo>("'hello'.nonexistent_method()").await.unwrap_err();
    assert!(matches!(err, crate::error::Error::PythonException(_)));
}

// -- The sandbox --

/// open() cannot reach the host filesystem: no directories are
/// preopened, so every path fails.
#[tokio::test]
async fn sandbox_no_filesystem() {
    let err = exec::<Foo>(r#"open("Cargo.toml").read()"#).await.unwrap_err();
    assert!(matches!(err, crate::error::Error::PythonException(_)));
}

/// The environment is empty: nothing from the host process leaks in.
#[tokio::test]
async fn sandbox_empty_environ() {
    let result: serde_json::Value = exec(
        r#"
import os
dict(os.environ)
"#,
    )
    .await
    .unwrap().unwrap();
    assert_eq!(result, serde_json::json!({}));
}

// -- PythonDeserialize errors --

/// Bare integer expression — not deserializable as Foo.
#[tokio::test]
async fn error_deser_bare_int() {
    let err = exec::<Foo>("42").await.unwrap_err();
    assert!(matches!(err, crate::error::Error::PythonDeserialize(_)));
}

/// Bare string expression — json.dumps wraps it in quotes, not a Foo.
#[tokio::test]
async fn error_deser_bare_string() {
    let err = exec::<Foo>(r#""hello""#).await.unwrap_err();
    assert!(matches!(err, crate::error::Error::PythonDeserialize(_)));
}

/// Bare list expression — wrong JSON shape for Foo.
#[tokio::test]
async fn error_deser_bare_list() {
    let err = exec::<Foo>("[1, 2, 3]").await.unwrap_err();
    assert!(matches!(err, crate::error::Error::PythonDeserialize(_)));
}

/// Bare `None` expression — `eval` serializes to JSON `null`, which the
/// harness treats as "no trailing expression"; stdout is empty too, so the
/// envelope carries no usable output and `exec` yields `Ok(None)`. This is
/// distinct from an explicit JSON `null` printed to stdout, which yields
/// `Ok(Some(Value::Null))` — see `output_explicit_json_null`.
#[tokio::test]
async fn no_output_bare_none() {
    let out = exec::<serde_json::Value>("None").await.unwrap();
    assert_eq!(out, None);
}

/// Print Python repr (single quotes) instead of JSON — not valid JSON.
#[tokio::test]
async fn error_deser_python_repr_not_json() {
    let err = exec::<Foo>(r#"print({"foo": "bar"})"#).await.unwrap_err();
    assert!(matches!(err, crate::error::Error::PythonDeserialize(_)));
}

/// Valid JSON but wrong shape — has "baz" key instead of "foo".
#[tokio::test]
async fn error_deser_wrong_shape() {
    let err = exec::<Foo>(r#"{"baz": "bar"}"#).await.unwrap_err();
    assert!(matches!(err, crate::error::Error::PythonDeserialize(_)));
}

/// Empty code — no output at all → `Ok(None)`.
#[tokio::test]
async fn no_output_empty_code() {
    let out = exec::<serde_json::Value>("").await.unwrap();
    assert_eq!(out, None);
}

/// Only comments — no output → `Ok(None)`.
#[tokio::test]
async fn no_output_only_comments() {
    let out = exec::<serde_json::Value>("# nothing here").await.unwrap();
    assert_eq!(out, None);
}

/// Explicit JSON `null` on stdout — distinct from no output: the envelope
/// carries a value, so `exec` yields `Ok(Some(Value::Null))`.
#[tokio::test]
async fn output_explicit_json_null() {
    let out = exec::<serde_json::Value>("import json; print(json.dumps(None))")
        .await
        .unwrap();
    assert_eq!(out, Some(serde_json::Value::Null));
}

/// Print valid JSON but an array of Foos, not a single Foo.
#[tokio::test]
async fn error_deser_array_of_foos() {
    let err = exec::<Foo>(
        r#"
import json
print(json.dumps([{"foo": "bar"}, {"foo": "baz"}]))
"#,
    )
    .await
    .unwrap_err();
    assert!(matches!(err, crate::error::Error::PythonDeserialize(_)));
}

/// Bare boolean expression — not a Foo.
#[tokio::test]
async fn error_deser_bare_bool() {
    let err = exec::<Foo>("True").await.unwrap_err();
    assert!(matches!(err, crate::error::Error::PythonDeserialize(_)));
}

/// Bare float expression — not a Foo.
#[tokio::test]
async fn error_deser_bare_float() {
    let err = exec::<Foo>("3.14").await.unwrap_err();
    assert!(matches!(err, crate::error::Error::PythonDeserialize(_)));
}

/// Bare tuple expression — json.dumps turns it into a list, not a Foo.
#[tokio::test]
async fn error_deser_bare_tuple() {
    let err = exec::<Foo>("(1, 2, 3)").await.unwrap_err();
    assert!(matches!(err, crate::error::Error::PythonDeserialize(_)));
}

// -- PythonFileRead errors --

/// Non-existent file.
#[tokio::test]
async fn error_file_not_found() {
    let (global, scoped) = ctx();
    let err = py()
        .await
        .exec_file::<(), Foo>(
            &global,
            &scoped,
            std::path::Path::new("/nonexistent/path/script.py"),
            None,
        )
        .await
        .unwrap_err();
    assert!(matches!(err, crate::error::Error::PythonFileRead(_, _)));
}
