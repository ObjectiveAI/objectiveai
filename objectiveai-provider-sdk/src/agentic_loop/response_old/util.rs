//! Accumulation helpers shared by the response types.

/// Appends an optional string to another.
///
/// The rule for every text field that streams: a delta is a fragment,
/// so fragments concatenate rather than replace.
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
