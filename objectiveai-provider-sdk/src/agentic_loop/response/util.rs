//! Accumulation helpers shared by the response types.

/// Adds an optional `u64` to another.
pub fn push_option_u64(self_value: &mut Option<u64>, other_value: &Option<u64>) {
    match (self_value.as_mut(), other_value) {
        (Some(self_value), Some(other_value)) => *self_value += other_value,
        (None, Some(other_value)) => *self_value = Some(*other_value),
        _ => {}
    }
}

/// Appends an optional string to another.
pub fn push_option_string(
    self_value: &mut Option<String>,
    other_value: &Option<String>,
) {
    match (self_value.as_mut(), other_value) {
        (Some(self_value), Some(other_value)) => self_value.push_str(other_value),
        (None, Some(other_value)) => *self_value = Some(other_value.clone()),
        _ => {}
    }
}
