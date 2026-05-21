//! Shared body input for every endpoint subcommand under `api/`.
//!
//! Each `api <...> {post,get,delete}` subcommand that carries a request body
//! flattens a `BodySource` so the same four flags work uniformly:
//!
//! - `--body-inline <json>`
//! - `--body-file <path>`
//! - `--body-python-inline <code>`
//! - `--body-python-file <path>`
//!
//! Errors from the JSON paths route through `serde_path_to_error` so a malformed
//! body points at the exact field that mismatched the typed request struct.
//! The python paths use `crate::python::exec_{code,file}`, which internally do
//! the same `serde_path_to_error` deserialize.

use clap::Args;

#[derive(Args)]
#[group(required = true, multiple = false)]
pub struct BodySource {
    /// Inline JSON body
    #[arg(long)]
    body_inline: Option<String>,
    /// Path to a JSON file
    #[arg(long)]
    body_file: Option<std::path::PathBuf>,
    /// Inline Python code that produces the JSON body
    #[arg(long)]
    body_python_inline: Option<String>,
    /// Path to a Python file that produces the JSON body
    #[arg(long)]
    body_python_file: Option<std::path::PathBuf>,
}

impl BodySource {
    pub fn resolve<T: serde::de::DeserializeOwned>(self) -> Result<T, crate::error::Error> {
        if let Some(inline) = self.body_inline {
            let mut de = serde_json::Deserializer::from_str(&inline);
            return serde_path_to_error::deserialize(&mut de)
                .map_err(crate::error::Error::InlineDeserialize);
        }
        if let Some(path) = self.body_file {
            let contents = std::fs::read_to_string(&path)
                .map_err(|e| crate::error::Error::PythonFileRead(path, e))?;
            let mut de = serde_json::Deserializer::from_str(&contents);
            return serde_path_to_error::deserialize(&mut de)
                .map_err(crate::error::Error::InlineDeserialize);
        }
        if let Some(code) = self.body_python_inline {
            return crate::python::exec_code(&code);
        }
        if let Some(path) = self.body_python_file {
            return crate::python::exec_file(&path);
        }
        unreachable!("clap group ensures one is set")
    }
}
