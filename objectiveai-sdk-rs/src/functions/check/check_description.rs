//! Description length validation for remote functions.

/// Maximum byte length for function descriptions (maps to GitHub repo description limit of 350 chars).
const MAX_DESCRIPTION_BYTES: usize = 350;

pub fn check_description(description: &str) -> Result<(), String> {
    if description.trim().is_empty() {
        return Err("QD01: Description must not be empty".to_string());
    }
    if description.len() > MAX_DESCRIPTION_BYTES {
        return Err(format!(
            "QD02: Description is {} bytes, exceeds maximum of {} bytes",
            description.len(),
            MAX_DESCRIPTION_BYTES
        ));
    }
    Ok(())
}
