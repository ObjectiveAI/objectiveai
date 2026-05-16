//! Distribution checks for output expressions.
//!
//! Verifies that a task's output expression maps the full range of raw
//! outputs to the full [0, 1] range (scalar) or produces well-distributed
//! per-element scores (vector). Catches expressions like `output * 0.1 + 0.45`
//! that pass uniqueness checks but produce a useless narrow range.
//!
//! Error codes:
//! - OD01: scalar output expression eval failed
//! - OD02: scalar output min bias (min > 0.01)
//! - OD03: scalar output max bias (max < 0.99)
//! - OD04: scalar output non-uniform (avg outside [0.4, 0.6])
//! - OD05: vector output expression eval failed
//! - OD06: vector element min bias
//! - OD07: vector element max bias
//! - OD08: vector element non-uniform
//! - OD09: mapped scalar output expression fails on all-zero inputs

use rust_decimal::Decimal;

use crate::functions::expression::{
    InputValue, TaskOutput, TaskOutputOwned,
};
use crate::functions::Task;

/// Number of systematic trials for distribution checks.
const TRIALS: usize = 1000;

/// Generates a systematic scalar value for trial k: sweeps 0.0 to 1.0.
fn systematic_scalar(k: usize) -> Decimal {
    Decimal::from_f64_retain(k as f64 / 999.0).unwrap_or(Decimal::ZERO)
}

/// Generates a systematic normalized vector of length `len` for trial k.
/// Trial 0 produces all zeros (stress-tests expressions that divide by sum).
/// Remaining trials use a focus-element strategy: one element at a time gets
/// most of the weight, sweeping from 0.0 to 1.0, ensuring per-element
/// extremes reach exactly 0 and 1 for any vector length.
fn systematic_vector(k: usize, len: usize) -> Vec<Decimal> {
    if len == 0 {
        return vec![];
    }
    if len == 1 {
        return vec![Decimal::ONE];
    }

    // Trial 0: all zeros — tests expressions that assume non-zero sums
    if k == 0 {
        return vec![Decimal::ZERO; len];
    }

    let k = k - 1; // shift remaining trials to 0-based
    let focus = k % len;
    let sub_k = k / len;
    let sub_trials = (TRIALS - 1) / len;
    let proportion = if sub_trials <= 1 {
        0.5
    } else {
        (sub_k as f64 / (sub_trials - 1) as f64).clamp(0.0, 1.0)
    };

    // Focused element gets `proportion` of total weight, rest is split evenly.
    // At proportion=1: focus=1.0, others=0.0  →  [0, ..., 1, ..., 0]
    // At proportion=0: focus=0.0, others=1/(len-1)
    let focus_weight = proportion;
    let other_weight = (1.0 - focus_weight) / (len - 1) as f64;

    (0..len)
        .map(|p| {
            let v = if p == focus {
                focus_weight
            } else {
                other_weight
            };
            Decimal::from_f64_retain(v).unwrap_or(Decimal::ZERO)
        })
        .collect()
}

/// Generates a systematic map-scalar output: a vec of scalars (not normalized).
/// Trial 0 produces all zeros (stress-tests expressions that divide by sum).
/// Remaining trials use a focus-element strategy: one element at a time gets a
/// high value while others get a minimal value, with the background scaling as
/// 1/len so that the focused element can reach near 1.0 after normalization
/// regardless of the number of elements.
fn systematic_map_scalar(k: usize, len: usize) -> Vec<Decimal> {
    if len == 0 {
        return vec![];
    }
    if len == 1 {
        let v = k as f64 / 999.0;
        return vec![
            Decimal::from_f64_retain(v).unwrap_or(Decimal::ZERO),
        ];
    }

    // Trial 0: all zeros — tests expressions that assume non-zero sums
    if k == 0 {
        return vec![Decimal::ZERO; len];
    }

    let k = k - 1;
    let focus = k % len;
    let sub_k = k / len;
    let sub_trials = (TRIALS - 1) / len;
    let proportion = if sub_trials <= 1 {
        0.5
    } else {
        (sub_k as f64 / (sub_trials - 1) as f64).clamp(0.0, 1.0)
    };

    // Background scales as 1/len so total background ≈ 0.001 regardless of len,
    // ensuring the focus element can always reach near 1.0 after normalization.
    let background = 0.001 / len as f64;
    let focus_value = proportion; // sweeps 0.0 → 1.0

    (0..len)
        .map(|i| {
            let v = if i == focus { focus_value } else { background };
            Decimal::from_f64_retain(v).unwrap_or(Decimal::ZERO)
        })
        .collect()
}

/// Generates a systematic scores vector with `n` elements.
fn systematic_vc_scores(k: usize, n: usize) -> Vec<Decimal> {
    systematic_vector(k, n)
}

// ---------------------------------------------------------------------------
// Scalar distribution check
// ---------------------------------------------------------------------------

/// Shape descriptor for the raw output to generate.
pub(crate) enum ScalarOutputShape {
    /// Raw output is a scalar function result.
    Scalar,
    /// Raw output is a vector completion with `n` responses.
    VectorCompletion(usize),
}

/// Runs 1000 systematic trials on a task's output expression and checks that
/// the compiled scalar output covers the full [0, 1] range uniformly.
///
/// Returns `Ok(())` if distribution is healthy, or `Err(message)` with a
/// user-facing error message.
pub(crate) fn check_scalar_distribution(
    task_index: usize,
    input: &InputValue,
    task: &Task,
    shape: &ScalarOutputShape,
) -> Result<(), String> {
    let mut min = Decimal::ONE;
    let mut max = Decimal::ZERO;
    let mut sum = Decimal::ZERO;

    for k in 0..TRIALS {
        let mock_output = match shape {
            ScalarOutputShape::Scalar => TaskOutput::Owned(
                TaskOutputOwned::Scalar(systematic_scalar(k)),
            ),
            ScalarOutputShape::VectorCompletion(n) => TaskOutput::Owned(
                TaskOutputOwned::Vector(systematic_vc_scores(k, *n)),
            ),
        };

        let result = task.compile_output(input, mock_output).map_err(|e| {
            format!(
                "OD01: Task [{}]: output expression evaluation failed during \
                 distribution check (trial {}): {}",
                task_index, k, e
            )
        })?;

        let value = match result {
            TaskOutputOwned::Scalar(s) => s,
            _ => {
                // Type mismatch is caught by CV validation; skip here
                return Ok(());
            }
        };

        if value < min {
            min = value;
        }
        if value > max {
            max = value;
        }
        sum += value;
    }

    let avg =
        sum / Decimal::from_f64_retain(TRIALS as f64).unwrap_or(Decimal::ONE);

    if min > Decimal::new(1, 2) {
        return Err(format!(
            "OD02: Task [{}]: output expression is biased — when given raw outputs \
             ranging from 0.0 to 1.0, the minimum compiled output was {} (expected \
             near 0.0). The output expression should map the full range of raw task \
             outputs to the full [0, 1] range.",
            task_index, min
        ));
    }
    if max < Decimal::new(99, 2) {
        return Err(format!(
            "OD03: Task [{}]: output expression is biased — when given raw outputs \
             ranging from 0.0 to 1.0, the maximum compiled output was {} (expected \
             near 1.0). The output expression should map the full range of raw task \
             outputs to the full [0, 1] range.",
            task_index, max
        ));
    }
    if avg < Decimal::new(4, 1) || avg > Decimal::new(6, 1) {
        return Err(format!(
            "OD04: Task [{}]: output expression is non-uniform — the average compiled \
             output was {} (expected near 0.5). The output expression should produce \
             roughly uniform results across the [0, 1] range when given uniformly \
             distributed raw outputs.",
            task_index, avg
        ));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Vector distribution check
// ---------------------------------------------------------------------------

/// Shape descriptor for the raw output to generate for vector checks.
pub(crate) enum VectorOutputShape {
    /// Raw output is a mapped scalar function: vec of scalars (not normalized).
    MapScalar(usize),
    /// Raw output is an unmapped vector function with given output_length.
    Vector(u64),
    /// Raw output is a vector completion with `n` responses.
    VectorCompletion(usize),
}

/// Runs 1000 systematic trials on a task's output expression and checks that
/// the compiled vector output has well-distributed per-element scores.
///
/// Returns `Ok(())` if distribution is healthy, or `Err(message)` with a
/// user-facing error message.
pub(crate) fn check_vector_distribution(
    task_index: usize,
    input: &InputValue,
    task: &Task,
    shape: &VectorOutputShape,
    output_length: usize,
) -> Result<(), String> {
    let mut per_element_min = vec![Decimal::ONE; output_length];
    let mut per_element_max = vec![Decimal::ZERO; output_length];
    let mut per_element_sum = vec![Decimal::ZERO; output_length];

    for k in 0..TRIALS {
        let mock_output = match shape {
            VectorOutputShape::MapScalar(len) => TaskOutput::Owned(
                TaskOutputOwned::Vector(systematic_map_scalar(k, *len)),
            ),
            VectorOutputShape::Vector(n) => TaskOutput::Owned(
                TaskOutputOwned::Vector(
                    systematic_vector(k, *n as usize),
                ),
            ),
            VectorOutputShape::VectorCompletion(n) => TaskOutput::Owned(
                TaskOutputOwned::Vector(systematic_vc_scores(k, *n)),
            ),
        };

        let result = task.compile_output(input, mock_output).map_err(|e| {
            // Special error for all-zero mapped scalar inputs (trial 0)
            if k == 0 && matches!(shape, VectorOutputShape::MapScalar(_)) {
                format!(
                    "OD09: Task [{}]: output expression fails when all mapped \
                     scalar outputs are zero (division by zero). The expression \
                     must handle the case where sum(output) == 0, e.g. use \
                     `x / sum(output) if sum(output) > 0 else 1.0 / len(output)`.",
                    task_index
                )
            } else {
                format!(
                    "OD05: Task [{}]: output expression evaluation failed during \
                     distribution check (trial {}): {}",
                    task_index, k, e
                )
            }
        })?;

        let values = match result {
            TaskOutputOwned::Vector(v) => v,
            _ => {
                // Type mismatch is caught by CV validation; skip here
                return Ok(());
            }
        };

        if values.len() != output_length {
            // Length mismatch is caught by CV validation; skip here
            return Ok(());
        }

        for (p, &v) in values.iter().enumerate() {
            if v < per_element_min[p] {
                per_element_min[p] = v;
            }
            if v > per_element_max[p] {
                per_element_max[p] = v;
            }
            per_element_sum[p] += v;
        }
    }

    let trials_dec =
        Decimal::from_f64_retain(TRIALS as f64).unwrap_or(Decimal::ONE);
    let expected_avg = Decimal::ONE
        / Decimal::from_f64_retain(output_length as f64)
            .unwrap_or(Decimal::ONE);
    let avg_low = expected_avg * Decimal::new(8, 1); // expected * 0.8
    let avg_high = expected_avg * Decimal::new(12, 1); // expected * 1.2

    for p in 0..output_length {
        let avg = per_element_sum[p] / trials_dec;

        if per_element_min[p] > Decimal::new(1, 2) {
            return Err(format!(
                "OD06: Task [{}], element [{}]: output expression is biased — when \
                 given systematic raw outputs, the minimum compiled value for this \
                 element was {} (expected near 0.0). Each element of the output vector \
                 should be able to reach values near 0.",
                task_index, p, per_element_min[p]
            ));
        }
        if per_element_max[p] < Decimal::new(99, 2) {
            return Err(format!(
                "OD07: Task [{}], element [{}]: output expression is biased — when \
                 given systematic raw outputs, the maximum compiled value for this \
                 element was {} (expected near 1.0). Each element of the output vector \
                 should be able to reach values near 1.",
                task_index, p, per_element_max[p]
            ));
        }
        if avg < avg_low || avg > avg_high {
            return Err(format!(
                "OD08: Task [{}], element [{}]: output expression is non-uniform — \
                 the average compiled value for this element was {} (expected near \
                 {:.4}). Each element should average around 1/{} when given uniformly \
                 distributed raw outputs.",
                task_index, p, avg, expected_avg, output_length
            ));
        }
    }

    Ok(())
}
