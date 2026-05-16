//! Utility functions for accumulating response data.

/// Adds an optional u64 value to another optional u64.
pub fn push_option_u64(
    self_value: &mut Option<u64>,
    other_value: &Option<u64>,
) {
    match (self_value.as_mut(), other_value) {
        (Some(self_value), Some(other_value)) => {
            *self_value += other_value;
        }
        (None, Some(other_value)) => {
            *self_value = Some(*other_value);
        }
        _ => {}
    }
}

/// Adds an optional decimal value to another optional decimal.
pub fn push_option_decimal(
    self_value: &mut Option<rust_decimal::Decimal>,
    other_value: &Option<rust_decimal::Decimal>,
) {
    match (self_value.as_mut(), other_value) {
        (Some(self_value), Some(other_value)) => {
            *self_value += other_value;
        }
        (None, Some(other_value)) => {
            *self_value = Some(*other_value);
        }
        _ => {}
    }
}

/// Appends an optional string to another optional string.
pub fn push_option_string(
    self_value: &mut Option<String>,
    other_value: &Option<String>,
) {
    match (self_value.as_mut(), other_value) {
        (Some(self_value), Some(other_value)) => {
            self_value.push_str(other_value);
        }
        (None, Some(other_value)) => {
            *self_value = Some(other_value.clone());
        }
        _ => {}
    }
}

/// Extends an optional vector with another optional vector.
pub fn push_option_vec<T: Clone>(
    self_value: &mut Option<Vec<T>>,
    other_value: &Option<Vec<T>>,
) {
    match (self_value.as_mut(), other_value) {
        (Some(self_value), Some(other_value)) => {
            self_value.extend(other_value.clone());
        }
        (None, Some(other_value)) => {
            *self_value = Some(other_value.clone());
        }
        _ => {}
    }
}
