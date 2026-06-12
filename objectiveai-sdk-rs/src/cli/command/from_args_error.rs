//! Error returned by `TryFrom<Args> for Request` impls on cli leaves.
//!
//! `field` names the `Args` field whose value failed to convert into
//! the leaf's typed `Request` form (e.g. `"path"`, `"agent_inline"`,
//! `"dangerous_advanced"`). `source` carries the specific failure —
//! either a JSON deserialization error (with `serde_path_to_error`
//! preserving the in-document location), or a plain `String` for the
//! docker-style `key=value,...` paths parsed by
//! [`super::path_ref::PathRef`].

#[derive(Debug)]
pub struct FromArgsError {
    pub field: &'static str,
    pub source: FromArgsErrorSource,
}

impl FromArgsError {
    /// Build a `FromArgsError` from a docker-style path parse error
    /// produced by [`super::path_ref::PathRef::from_str`] or the
    /// [`FromStr`] impl on [`crate::RemotePathCommitOptional`].
    ///
    /// [`FromStr`]: std::str::FromStr
    pub fn path_parse(field: &'static str, source: String) -> Self {
        Self {
            field,
            source: FromArgsErrorSource::Plain(source),
        }
    }

    /// Build a `FromArgsError` from a `serde_path_to_error`-wrapped
    /// JSON deserialization failure. Existing call sites that built
    /// `FromArgsError { field, source }` with a `serde_path_to_error`
    /// error keep working — `source` accepts both shapes via the
    /// `From<serde_path_to_error::Error<serde_json::Error>>` impl.
    pub fn json(
        field: &'static str,
        source: serde_path_to_error::Error<serde_json::Error>,
    ) -> Self {
        Self {
            field,
            source: FromArgsErrorSource::Json(source),
        }
    }
}

#[derive(Debug)]
pub enum FromArgsErrorSource {
    Json(serde_path_to_error::Error<serde_json::Error>),
    Plain(String),
}

impl From<serde_path_to_error::Error<serde_json::Error>> for FromArgsErrorSource {
    fn from(e: serde_path_to_error::Error<serde_json::Error>) -> Self {
        Self::Json(e)
    }
}

impl From<String> for FromArgsErrorSource {
    fn from(s: String) -> Self {
        Self::Plain(s)
    }
}

impl std::fmt::Display for FromArgsErrorSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json(e) => write!(f, "{e}"),
            Self::Plain(s) => f.write_str(s),
        }
    }
}

impl std::fmt::Display for FromArgsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "argument parse error at `{}`: {}", self.field, self.source)
    }
}

impl std::error::Error for FromArgsError {}
