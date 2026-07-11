use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// One whitelist row. Each field is a regex matched **anchored**
/// against the corresponding install parameter (compiled as
/// `^(?:pattern)$`). All four fields must match for the row to
/// allow the install.
///
/// The host loads a list of these and accepts an install only
/// if the (owner, repository, commit_sha_or_HEAD, manifest.version)
/// quadruple matches at least one row. Use `.*` for catch-all fields.
/// Shared by the tool and plugin GitHub-install paths.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "filesystem.plugins.WhitelistEntry")]
pub struct WhitelistEntry {
    /// Regex matching the GitHub owner (org or user). Case-sensitive.
    pub owner: String,
    /// Regex matching the repository name.
    pub repository: String,
    /// Regex matching the commit sha. When the user omits
    /// `--commit-sha`, this is matched against the literal string
    /// `"HEAD"`.
    pub commit_sha: String,
    /// Regex matching the manifest's `version` field.
    pub version: String,
}

/// The hard-coded default whitelist. Single entry: any repo under
/// the ObjectiveAI GitHub org passes; everything else requires
/// `--allow-untrusted`.
pub fn default_whitelist() -> Vec<WhitelistEntry> {
    vec![WhitelistEntry {
        // (?i) is the regex inline flag for case-insensitive matching;
        // GitHub usernames are case-insensitive at the URL level, so
        // accept any capitalization of the canonical org name.
        owner: "(?i)ObjectiveAI".to_string(),
        repository: ".*".to_string(),
        commit_sha: ".*".to_string(),
        version: ".*".to_string(),
    }]
}

/// Returns `Ok(true)` iff the quadruple matches at least one entry
/// in `whitelist`. Each field is compiled as `^(?:pattern)$` so
/// patterns like `Object` won't match `ObjectAttacker`. Returns
/// `Err(regex::Error)` if any pattern fails to compile.
pub fn check_plugin_whitelist(
    owner: &str,
    repository: &str,
    commit_sha: &str,
    version: &str,
    whitelist: &[WhitelistEntry],
) -> Result<bool, regex::Error> {
    for entry in whitelist {
        if anchored_match(&entry.owner, owner)?
            && anchored_match(&entry.repository, repository)?
            && anchored_match(&entry.commit_sha, commit_sha)?
            && anchored_match(&entry.version, version)?
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn anchored_match(pattern: &str, value: &str) -> Result<bool, regex::Error> {
    let anchored = format!("^(?:{pattern})$");
    let re = regex::Regex::new(&anchored)?;
    Ok(re.is_match(value))
}
