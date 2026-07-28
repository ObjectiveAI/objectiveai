//! Arbitrary helpers for types that don't natively implement `Arbitrary`
//! or that need bounded generation to avoid extreme values.

fn arbitrary_option<T>(
    u: &mut arbitrary::Unstructured,
    arbitrary_value: impl Fn(&mut arbitrary::Unstructured) -> arbitrary::Result<T>,
) -> arbitrary::Result<Option<T>> {
    if u.arbitrary().unwrap_or(false) {
        Ok(Some(arbitrary_value(u)?))
    } else {
        Ok(None)
    }
}

fn arbitrary_hash_map<K: std::hash::Hash + Eq, V>(
    u: &mut arbitrary::Unstructured,
    arbitrary_key: impl Fn(&mut arbitrary::Unstructured) -> arbitrary::Result<K>,
    arbitrary_value: impl Fn(&mut arbitrary::Unstructured) -> arbitrary::Result<V>,
) -> arbitrary::Result<std::collections::HashMap<K, V>> {
    let mut map = std::collections::HashMap::new();
    while u.arbitrary().unwrap_or(false) {
        map.insert(arbitrary_key(u)?, arbitrary_value(u)?);
    }
    Ok(map)
}

fn arbitrary_index_map<K: std::hash::Hash + Eq, V>(
    u: &mut arbitrary::Unstructured,
    arbitrary_key: impl Fn(&mut arbitrary::Unstructured) -> arbitrary::Result<K>,
    arbitrary_value: impl Fn(&mut arbitrary::Unstructured) -> arbitrary::Result<V>,
) -> arbitrary::Result<indexmap::IndexMap<K, V>> {
    let mut map = indexmap::IndexMap::new();
    while u.arbitrary().unwrap_or(false) {
        map.insert(arbitrary_key(u)?, arbitrary_value(u)?);
    }
    Ok(map)
}

fn arbitrary_vec<T>(
    u: &mut arbitrary::Unstructured,
    arbitrary_value: impl Fn(&mut arbitrary::Unstructured) -> arbitrary::Result<T>,
) -> arbitrary::Result<Vec<T>> {
    let mut v = Vec::new();
    while u.arbitrary().unwrap_or(false) {
        v.push(arbitrary_value(u)?);
    }
    Ok(v)
}

/// Generates an arbitrary `rust_decimal::Decimal` from an f32,
/// scaled down by 10^13. f32::MAX is ~3.4e38, so every finite
/// sample lands at ≤ ~3.4e25 — a hard ceiling ~2,300× below
/// `Decimal::MAX` (~7.9e28), leaving ample headroom for the
/// chunk-push suites that sum ~20 of these through `Usage::push`
/// (`Decimal::AddAssign` panics on overflow). `Inf`/`NaN` samples
/// stay non-finite after division and fall back to zero via
/// `unwrap_or_default`.
pub fn arbitrary_rust_decimal(
    u: &mut arbitrary::Unstructured,
) -> arbitrary::Result<rust_decimal::Decimal> {
    let f: f32 = u.arbitrary()?;
    Ok(rust_decimal::Decimal::try_from(f / 1e13).unwrap_or_default())
}

/// Generates an arbitrary `Option<rust_decimal::Decimal>`.
pub fn arbitrary_option_rust_decimal(
    u: &mut arbitrary::Unstructured,
) -> arbitrary::Result<Option<rust_decimal::Decimal>> {
    arbitrary_option(u, arbitrary_rust_decimal)
}

/// Generates an arbitrary `Vec<rust_decimal::Decimal>`.
pub fn arbitrary_vec_rust_decimal(
    u: &mut arbitrary::Unstructured,
) -> arbitrary::Result<Vec<rust_decimal::Decimal>> {
    arbitrary_vec(u, arbitrary_rust_decimal)
}

/// Generates an arbitrary `Vec<Vec<rust_decimal::Decimal>>`.
pub fn arbitrary_vec_vec_rust_decimal(
    u: &mut arbitrary::Unstructured,
) -> arbitrary::Result<Vec<Vec<rust_decimal::Decimal>>> {
    arbitrary_vec(u, arbitrary_vec_rust_decimal)
}

/// Generates an arbitrary `f64` from an f32 (bounded range).
pub fn arbitrary_f64(
    u: &mut arbitrary::Unstructured,
) -> arbitrary::Result<f64> {
    Ok(u.arbitrary::<f32>()? as f64)
}

/// Generates an arbitrary `Option<f64>` (bounded range).
pub fn arbitrary_option_f64(
    u: &mut arbitrary::Unstructured,
) -> arbitrary::Result<Option<f64>> {
    arbitrary_option(u, arbitrary_f64)
}

/// Generates an arbitrary `u64` from a u32 (bounded range).
pub fn arbitrary_u64(
    u: &mut arbitrary::Unstructured,
) -> arbitrary::Result<u64> {
    Ok(u.arbitrary::<u32>()? as u64)
}

/// Generates an arbitrary `usize` from a u16 (bounded range).
pub fn arbitrary_usize(
    u: &mut arbitrary::Unstructured,
) -> arbitrary::Result<usize> {
    Ok(u.arbitrary::<u16>()? as usize)
}

/// Generates an arbitrary `isize` from an i16 (bounded range).
pub fn arbitrary_isize(
    u: &mut arbitrary::Unstructured,
) -> arbitrary::Result<isize> {
    Ok(u.arbitrary::<i16>()? as isize)
}

/// Generates an arbitrary `Option<usize>` (bounded range).
pub fn arbitrary_option_usize(
    u: &mut arbitrary::Unstructured,
) -> arbitrary::Result<Option<usize>> {
    arbitrary_option(u, arbitrary_usize)
}

/// Generates an arbitrary `Option<isize>` (bounded range).
pub fn arbitrary_option_isize(
    u: &mut arbitrary::Unstructured,
) -> arbitrary::Result<Option<isize>> {
    arbitrary_option(u, arbitrary_isize)
}

/// Generates an arbitrary `Vec<u64>` with bounded elements.
pub fn arbitrary_vec_u64(
    u: &mut arbitrary::Unstructured,
) -> arbitrary::Result<Vec<u64>> {
    arbitrary_vec(u, arbitrary_u64)
}

/// Generates an arbitrary `IndexMap<K, V>` where both K and V implement Arbitrary.
pub fn arbitrary_indexmap<'a, K, V>(
    u: &mut arbitrary::Unstructured<'a>,
) -> arbitrary::Result<indexmap::IndexMap<K, V>>
where
    K: arbitrary::Arbitrary<'a> + std::hash::Hash + Eq,
    V: arbitrary::Arbitrary<'a>,
{
    let mut map = indexmap::IndexMap::new();
    while u.arbitrary().unwrap_or(false) {
        map.insert(K::arbitrary(u)?, V::arbitrary(u)?);
    }
    Ok(map)
}

/// Generates an arbitrary `Option<IndexMap<String, i64>>`.
pub fn arbitrary_option_indexmap_string_i64(
    u: &mut arbitrary::Unstructured,
) -> arbitrary::Result<Option<indexmap::IndexMap<String, i64>>> {
    arbitrary_option(u, |u| {
        arbitrary_index_map(u, |u| u.arbitrary(), arbitrary_i64)
    })
}

/// Generates an arbitrary `Option<IndexMap<String, Option<String>>>`.
pub fn arbitrary_option_indexmap_string_option_string(
    u: &mut arbitrary::Unstructured,
) -> arbitrary::Result<Option<indexmap::IndexMap<String, Option<String>>>> {
    arbitrary_option(u, |u| {
        arbitrary_index_map(u, |u| u.arbitrary(), |u| u.arbitrary())
    })
}

/// Generates an arbitrary `Option<IndexMap<String, serde_json::Value>>`
/// — plugin arguments, whose values are free-form JSON.
pub fn arbitrary_option_indexmap_string_json_value(
    u: &mut arbitrary::Unstructured,
) -> arbitrary::Result<Option<indexmap::IndexMap<String, serde_json::Value>>> {
    arbitrary_option(u, |u| {
        arbitrary_index_map(u, |u| u.arbitrary(), arbitrary_json_value)
    })
}

/// Generates an arbitrary `Option<u64>` (bounded range).
pub fn arbitrary_option_u64(
    u: &mut arbitrary::Unstructured,
) -> arbitrary::Result<Option<u64>> {
    arbitrary_option(u, arbitrary_u64)
}

/// Generates an arbitrary `i64` from an i32 (bounded range).
pub fn arbitrary_i64(
    u: &mut arbitrary::Unstructured,
) -> arbitrary::Result<i64> {
    Ok(u.arbitrary::<i32>()? as i64)
}

/// Generates an arbitrary `Option<i64>` (bounded range).
pub fn arbitrary_option_i64(
    u: &mut arbitrary::Unstructured,
) -> arbitrary::Result<Option<i64>> {
    arbitrary_option(u, arbitrary_i64)
}

/// Generates an arbitrary `Vec<i64>` (bounded range).
pub fn arbitrary_vec_i64(
    u: &mut arbitrary::Unstructured,
) -> arbitrary::Result<Vec<i64>> {
    arbitrary_vec(u, arbitrary_i64)
}

/// Generates an arbitrary `Option<Vec<i64>>` (bounded range).
pub fn arbitrary_option_vec_i64(
    u: &mut arbitrary::Unstructured,
) -> arbitrary::Result<Option<Vec<i64>>> {
    arbitrary_option(u, arbitrary_vec_i64)
}

/// Generates an arbitrary `serde_json::Value`. Nesting requires a "go deep"
/// bool to be true (geometric distribution), so compound structures are
/// exponentially rare. Collection sizes also use bool-per-element.
pub fn arbitrary_json_value(
    u: &mut arbitrary::Unstructured,
) -> arbitrary::Result<serde_json::Value> {
    if u.arbitrary().unwrap_or(false) {
        // Compound value
        if u.arbitrary()? {
            let mut arr = Vec::new();
            while u.arbitrary().unwrap_or(false) {
                arr.push(arbitrary_json_value(u)?);
            }
            Ok(serde_json::Value::Array(arr))
        } else {
            let mut map = serde_json::Map::new();
            while u.arbitrary().unwrap_or(false) {
                let key: String = u.arbitrary()?;
                map.insert(key, arbitrary_json_value(u)?);
            }
            Ok(serde_json::Value::Object(map))
        }
    } else {
        // Leaf value
        match u.int_in_range(0..=3)? {
            0 => Ok(serde_json::Value::Null),
            1 => Ok(serde_json::Value::Bool(u.arbitrary()?)),
            2 => Ok(serde_json::Value::Number(serde_json::Number::from(
                u.arbitrary::<i32>()?,
            ))),
            _ => Ok(serde_json::Value::String(u.arbitrary()?)),
        }
    }
}
