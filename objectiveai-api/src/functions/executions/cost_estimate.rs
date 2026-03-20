//! Cost estimation for Function executions.
//!
//! Given historical usage for a Function/Profile pair and a prospective
//! input, this module produces a statistical cost range with a confidence
//! score. The implementation is intentionally conservative and based on
//! aggregate usage statistics only – it **does not** attempt to model
//! provider‑level pricing details.

use serde::{Deserialize, Serialize};

/// Size information for a single top‑level input field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldSize {
    /// Name of the top‑level field.
    pub name: String,
    /// Approximate character count for the field's value.
    pub chars: u64,
}

/// Aggregate size statistics for the provided input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputSizeStats {
    /// Approximate total character count for the entire input.
    pub total_chars: u64,
    /// Approximate character counts for each top‑level field.
    pub top_level_fields: Vec<FieldSize>,
}

/// Statistical cost estimate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEstimateRange {
    /// Mean estimated cost in credits.
    pub mean: rust_decimal::Decimal,
    /// Lower bound of the estimated cost range.
    pub lower: rust_decimal::Decimal,
    /// Upper bound of the estimated cost range.
    pub upper: rust_decimal::Decimal,
    /// Heuristic confidence in the estimate in \[0, 1\].
    pub confidence: f32,
}

/// Request body for estimating Function execution cost.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEstimateRequestBody {
    /// The input used to estimate Function execution cost.
    pub input: objectiveai::functions::expression::Input,
}

/// Response payload for a Function execution cost estimate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEstimateResponse {
    /// Function GitHub owner.
    pub function_owner: String,
    /// Function GitHub repository.
    pub function_repository: String,
    /// Optional Function commit SHA (if provided in the request path).
    pub function_commit: Option<String>,
    /// Profile GitHub owner.
    pub profile_owner: String,
    /// Profile GitHub repository.
    pub profile_repository: String,
    /// Optional Profile commit SHA (if provided in the request path).
    pub profile_commit: Option<String>,
    /// Aggregate size information for the provided input.
    pub input_size: InputSizeStats,
    /// Historical usage statistics for this Function/Profile pair.
    pub usage: objectiveai::functions::response::UsageFunctionProfilePair,
    /// Estimated cost range and confidence.
    pub estimate: CostEstimateRange,
}

/// Computes a cost estimate (size stats + range) from historical usage and input.
///
/// This uses a very simple model:
/// - Derive a **cost per token** from historical usage
/// - Approximate prompt tokens from the current input size
/// - Use historical average completion tokens per request
/// - Expand the mean cost into a symmetric range whose width and confidence
///   depend on the amount of historical data available
pub fn estimate_cost(
    usage: &objectiveai::functions::response::UsageFunctionProfilePair,
    input: &objectiveai::functions::expression::Input,
) -> (InputSizeStats, CostEstimateRange) {
    use rust_decimal::Decimal;

    let input_size = InputSizeStats {
        total_chars: estimate_chars(input),
        top_level_fields: estimate_top_level_fields(input),
    };

    // Default to zero cost and zero confidence if we have no usable history.
    if usage.requests == 0
        || (usage.prompt_tokens == 0 && usage.completion_tokens == 0)
        || usage.total_cost.is_zero()
    {
        return (
            input_size,
            CostEstimateRange {
                mean: Decimal::ZERO,
                lower: Decimal::ZERO,
                upper: Decimal::ZERO,
                confidence: 0.0,
            },
        );
    }

    let total_tokens =
        usage.prompt_tokens.saturating_add(usage.completion_tokens) as u64;
    let total_tokens_dec = Decimal::from(total_tokens);

    // Cost per token derived from historical usage.
    let cost_per_token = usage.total_cost / total_tokens_dec;

    // Approximate prompt tokens from input size:
    // assume ~4 characters per token as a rough heuristic.
    let approx_prompt_tokens = if input_size.total_chars == 0 {
        Decimal::ZERO
    } else {
        Decimal::from(input_size.total_chars) / Decimal::from(4u64)
    };

    // Historical average completion tokens per request.
    let avg_completion_tokens = if usage.requests == 0 {
        Decimal::ZERO
    } else {
        Decimal::from(usage.completion_tokens)
            / Decimal::from(usage.requests as u64)
    };

    let predicted_total_tokens = approx_prompt_tokens + avg_completion_tokens;
    let mean = (predicted_total_tokens * cost_per_token).round_dp(6);

    // Confidence and range width based on the number of historical requests.
    let (range_fraction, confidence) = if usage.requests < 10 {
        // Very little history: wide interval, low confidence.
        (Decimal::new(50, 2), 0.5) // ±50%
    } else if usage.requests < 100 {
        // Moderate history: narrower interval, higher confidence.
        (Decimal::new(30, 2), 0.8) // ±30%
    } else {
        // Plenty of history: relatively tight interval, high confidence.
        (Decimal::new(20, 2), 0.9) // ±20%
    };

    let one = Decimal::ONE;
    let lower = (mean * (one - range_fraction)).max(Decimal::ZERO);
    let upper = mean * (one + range_fraction);

    let estimate = CostEstimateRange {
        mean,
        lower,
        upper,
        confidence,
    };

    (input_size, estimate)
}

/// Approximate character count for an input value.
fn estimate_chars(input: &objectiveai::functions::expression::Input) -> u64 {
    use objectiveai::functions::expression::Input;

    match input {
        Input::RichContentPart(_) => {
            // Rich content (images, files, etc.) are treated as a small
            // fixed overhead – the actual cost impact is dominated by
            // metadata rather than full binary contents.
            64
        }
        Input::Object(map) => {
            let mut total = 2; // braces
            for (key, value) in map.iter() {
                total += key.len() as u64;
                total += estimate_chars(value);
            }
            total
        }
        Input::Array(items) => {
            let mut total = 2; // brackets
            for item in items {
                total += estimate_chars(item);
            }
            total
        }
        Input::String(s) => s.chars().count() as u64,
        Input::Integer(i) => i.to_string().len() as u64,
        Input::Number(n) => n.to_string().len() as u64,
        Input::Boolean(b) => {
            if *b {
                4
            } else {
                5
            }
        }
    }
}

/// Approximate size per top‑level field in an input object.
fn estimate_top_level_fields(
    input: &objectiveai::functions::expression::Input,
) -> Vec<FieldSize> {
    use objectiveai::functions::expression::Input;

    match input {
        Input::Object(map) => map
            .iter()
            .map(|(name, value)| FieldSize {
                name: name.clone(),
                chars: estimate_chars(value),
            })
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_cost_with_basic_usage_and_input() {
        use objectiveai::functions::expression::Input;
        use objectiveai::functions::response::UsageFunctionProfilePair;
        use rust_decimal::Decimal;

        let usage = UsageFunctionProfilePair {
            requests: 20,
            completion_tokens: 1_000,
            prompt_tokens: 2_000,
            total_cost: Decimal::new(50, 2), // 0.50 credits
        };

        let input = Input::Object(
            [(
                "text".to_string(),
                Input::String("hello world".to_string()),
            )]
            .into_iter()
            .collect(),
        );

        let (size, estimate) = estimate_cost(&usage, &input);

        // Sanity checks: non‑zero size, non‑zero mean, bounds ordered.
        assert!(size.total_chars > 0);
        assert!(estimate.mean > Decimal::ZERO);
        assert!(estimate.lower <= estimate.mean);
        assert!(estimate.upper >= estimate.mean);
        assert!(estimate.confidence > 0.0);
    }

    #[test]
    fn estimate_cost_without_usage_returns_zero() {
        use objectiveai::functions::expression::Input;
        use objectiveai::functions::response::UsageFunctionProfilePair;
        use rust_decimal::Decimal;

        let usage = UsageFunctionProfilePair {
            requests: 0,
            completion_tokens: 0,
            prompt_tokens: 0,
            total_cost: Decimal::ZERO,
        };

        let input = Input::String("irrelevant".to_string());

        let (_size, estimate) = estimate_cost(&usage, &input);

        assert!(estimate.mean.is_zero());
        assert!(estimate.lower.is_zero());
        assert!(estimate.upper.is_zero());
        assert_eq!(estimate.confidence, 0.0);
    }
}

