/// Tracks the lifetime of the temp directory holding a serialized output
/// schema. Mirrors `OutputSchemaFile` in `output_schema_file.py:13-39`
/// (`@dataclass(frozen=True)`).
///
/// Python names the second field `_dir` (underscore-prefix is its
/// internal-by-convention marker). Rust has no leading-underscore convention
/// so we name it `dir` and document the parallel.
///
/// The async `cleanup()` method on the Python type is runtime behavior and
/// not part of the data definition; that ports separately when we wire up
/// the upstream client.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputSchemaFile {
    pub schema_path: Option<String>,
    pub dir: Option<String>,
}
