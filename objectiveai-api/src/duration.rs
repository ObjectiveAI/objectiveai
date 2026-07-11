//! Duration billing math. BILLING-CRITICAL — keep the arithmetic exact.
//!
//! Rates are per-1-SECOND `Decimal`s (see `ctx::Context`'s per-upstream
//! `*_duration_cost` fields); elapsed times are integer milliseconds. The
//! charge is `Decimal::from(elapsed_ms) × rate ÷ Decimal::from(1000)` —
//! all exact base-10 operations (the ÷1000 only shifts the scale). Each
//! upstream client measures its own create→finish elapsed and adds this
//! charge to BOTH `cost` and `total_cost`, raw (no `cost_multiplier`) and
//! unconditionally (BYOK included): duration is ObjectiveAI infra time,
//! not a provider charge.

/// The exact charge for `elapsed_ms` at `rate_per_second`.
pub fn duration_charge(
    elapsed_ms: u64,
    rate_per_second: rust_decimal::Decimal,
) -> rust_decimal::Decimal {
    rust_decimal::Decimal::from(elapsed_ms) * rate_per_second
        / rust_decimal::Decimal::from(1000_u64)
}
