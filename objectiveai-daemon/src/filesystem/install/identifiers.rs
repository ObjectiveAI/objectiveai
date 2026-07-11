use super::{InstallError, InstallKind};

/// Reject reserved repository names before any install side-effect.
/// `objectiveai` (case-insensitive) is reserved because the viewer
/// uses it as the Tauri channel name for built-in events; a plugin
/// with that repository name would shadow them. Only applied to
/// plugin installs — tools have no such collision.
fn check_repository_name(repository: &str) -> Result<(), InstallError> {
    if repository.eq_ignore_ascii_case("objectiveai") {
        return Err(InstallError::ReservedRepositoryName {
            repository: repository.to_string(),
        });
    }
    Ok(())
}

/// Identifier shape check shared by `owner`, `repository`, and
/// `commit`: Anthropic's tool-name regex (`^[a-zA-Z0-9_-]{1,128}$`)
/// plus `.` (so semver-shaped versions and dotted commit refs flow
/// through cleanly; the `.` -> `-` substitution happens when the
/// LLM-visible tool name is materialized downstream).
fn validate_identifier(
    kind: &'static str,
    value: &str,
) -> Result<(), InstallError> {
    let valid_len = !value.is_empty() && value.len() <= 128;
    let valid_chars = value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'));
    if !valid_len || !valid_chars {
        return Err(InstallError::InvalidIdentifier {
            kind,
            value: value.to_string(),
        });
    }
    Ok(())
}

/// Combined shape check for the three caller-supplied identifiers every
/// install entry point takes. For [`InstallKind::Plugin`] the reserved
/// repository-name check runs first (so a reserved-name failure takes
/// precedence over a generic regex failure for the same input); tools
/// skip it.
pub(crate) fn validate_install_inputs(
    kind: InstallKind,
    owner: &str,
    repository: &str,
    commit_sha: Option<&str>,
) -> Result<(), InstallError> {
    if matches!(kind, InstallKind::Plugin) {
        check_repository_name(repository)?;
    }
    validate_identifier("owner", owner)?;
    validate_identifier("repository", repository)?;
    if let Some(sha) = commit_sha {
        validate_identifier("commit", sha)?;
    }
    Ok(())
}

/// Convention: the raw-GitHub URL we'd fetch `objectiveai.json` from
/// for a given (owner, repository, optional commit sha). Defaults to
/// `HEAD` when no commit is supplied. Lifted out so the cli and the
/// SDK's own install wrappers share one source of truth.
pub fn raw_manifest_url(
    owner: &str,
    repository: &str,
    commit_sha: Option<&str>,
) -> String {
    let reference = commit_sha.unwrap_or("HEAD");
    format!(
        "https://raw.githubusercontent.com/{owner}/{repository}/{reference}/objectiveai.json"
    )
}

pub(crate) fn build_headers(
    headers: Option<&indexmap::IndexMap<String, String>>,
) -> Result<reqwest::header::HeaderMap, InstallError> {
    let mut out = reqwest::header::HeaderMap::new();
    let Some(h) = headers else {
        return Ok(out);
    };
    for (k, v) in h {
        let name = reqwest::header::HeaderName::from_bytes(k.as_bytes())
            .map_err(|e| InstallError::InvalidHeaderName {
                name: k.clone(),
                reason: e.to_string(),
            })?;
        let value = reqwest::header::HeaderValue::from_str(v).map_err(|e| {
            InstallError::InvalidHeaderValue {
                name: k.clone(),
                reason: e.to_string(),
            }
        })?;
        out.insert(name, value);
    }
    Ok(out)
}
