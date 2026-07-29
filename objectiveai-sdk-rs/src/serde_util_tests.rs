//! Tests for `serde_util` numeric-only Decimal deserialization.

use rust_decimal::Decimal;
use serde::Deserialize;

/// Test struct exercising all four `deserialize_with` functions.
#[derive(Deserialize, Debug, PartialEq)]
struct All {
    #[serde(deserialize_with = "crate::serde_util::decimal")]
    bare: Decimal,
    #[serde(deserialize_with = "crate::serde_util::option_decimal")]
    opt: Option<Decimal>,
    #[serde(deserialize_with = "crate::serde_util::vec_decimal")]
    vec: Vec<Decimal>,
    #[serde(deserialize_with = "crate::serde_util::vec_vec_decimal")]
    vecs: Vec<Vec<Decimal>>,
}

// ---- bare decimal ----

#[test]
fn bare_integer() {
    #[derive(Deserialize)]
    struct S {
        #[serde(deserialize_with = "crate::serde_util::decimal")]
        v: Decimal,
    }
    let s: S = serde_json::from_str(r#"{"v": 42}"#).unwrap();
    assert_eq!(s.v, Decimal::from(42));
}

#[test]
fn bare_float() {
    #[derive(Deserialize)]
    struct S {
        #[serde(deserialize_with = "crate::serde_util::decimal")]
        v: Decimal,
    }
    let s: S = serde_json::from_str(r#"{"v": 3.14}"#).unwrap();
    assert!(s.v > Decimal::from(3) && s.v < Decimal::from(4));
}

#[test]
fn bare_negative() {
    #[derive(Deserialize)]
    struct S {
        #[serde(deserialize_with = "crate::serde_util::decimal")]
        v: Decimal,
    }
    let s: S = serde_json::from_str(r#"{"v": -99}"#).unwrap();
    assert_eq!(s.v, Decimal::from(-99));
}

#[test]
fn bare_zero() {
    #[derive(Deserialize)]
    struct S {
        #[serde(deserialize_with = "crate::serde_util::decimal")]
        v: Decimal,
    }
    let s: S = serde_json::from_str(r#"{"v": 0}"#).unwrap();
    assert_eq!(s.v, Decimal::ZERO);
}

#[test]
fn bare_rejects_string() {
    #[derive(Deserialize)]
    struct S {
        #[serde(deserialize_with = "crate::serde_util::decimal")]
        v: Decimal,
    }
    assert!(serde_json::from_str::<S>(r#"{"v": "94"}"#).is_err());
}

#[test]
fn bare_rejects_bool() {
    #[derive(Deserialize)]
    struct S {
        #[serde(deserialize_with = "crate::serde_util::decimal")]
        v: Decimal,
    }
    assert!(serde_json::from_str::<S>(r#"{"v": true}"#).is_err());
}

#[test]
fn bare_rejects_null() {
    #[derive(Deserialize)]
    struct S {
        #[serde(deserialize_with = "crate::serde_util::decimal")]
        v: Decimal,
    }
    assert!(serde_json::from_str::<S>(r#"{"v": null}"#).is_err());
}

#[test]
fn bare_rejects_array() {
    #[derive(Deserialize)]
    struct S {
        #[serde(deserialize_with = "crate::serde_util::decimal")]
        v: Decimal,
    }
    assert!(serde_json::from_str::<S>(r#"{"v": [1]}"#).is_err());
}

#[test]
fn bare_rejects_object() {
    #[derive(Deserialize)]
    struct S {
        #[serde(deserialize_with = "crate::serde_util::decimal")]
        v: Decimal,
    }
    assert!(serde_json::from_str::<S>(r#"{"v": {}}"#).is_err());
}

// ---- option decimal ----

#[test]
fn option_some_number() {
    #[derive(Deserialize)]
    struct S {
        #[serde(deserialize_with = "crate::serde_util::option_decimal")]
        v: Option<Decimal>,
    }
    let s: S = serde_json::from_str(r#"{"v": 7}"#).unwrap();
    assert_eq!(s.v, Some(Decimal::from(7)));
}

#[test]
fn option_null() {
    #[derive(Deserialize)]
    struct S {
        #[serde(deserialize_with = "crate::serde_util::option_decimal")]
        v: Option<Decimal>,
    }
    let s: S = serde_json::from_str(r#"{"v": null}"#).unwrap();
    assert_eq!(s.v, None);
}

#[test]
fn option_rejects_string() {
    #[derive(Deserialize)]
    struct S {
        #[serde(deserialize_with = "crate::serde_util::option_decimal")]
        v: Option<Decimal>,
    }
    assert!(serde_json::from_str::<S>(r#"{"v": "42"}"#).is_err());
}

// ---- vec decimal ----

#[test]
fn vec_numbers() {
    #[derive(Deserialize)]
    struct S {
        #[serde(deserialize_with = "crate::serde_util::vec_decimal")]
        v: Vec<Decimal>,
    }
    let s: S = serde_json::from_str(r#"{"v": [1, 2.5, -3]}"#).unwrap();
    assert_eq!(s.v.len(), 3);
    assert_eq!(s.v[0], Decimal::from(1));
    assert_eq!(s.v[2], Decimal::from(-3));
}

#[test]
fn vec_empty() {
    #[derive(Deserialize)]
    struct S {
        #[serde(deserialize_with = "crate::serde_util::vec_decimal")]
        v: Vec<Decimal>,
    }
    let s: S = serde_json::from_str(r#"{"v": []}"#).unwrap();
    assert!(s.v.is_empty());
}

#[test]
fn vec_rejects_string_element() {
    #[derive(Deserialize)]
    struct S {
        #[serde(deserialize_with = "crate::serde_util::vec_decimal")]
        v: Vec<Decimal>,
    }
    assert!(serde_json::from_str::<S>(r#"{"v": [1, "2", 3]}"#).is_err());
}

#[test]
fn vec_rejects_not_array() {
    #[derive(Deserialize)]
    struct S {
        #[serde(deserialize_with = "crate::serde_util::vec_decimal")]
        v: Vec<Decimal>,
    }
    assert!(serde_json::from_str::<S>(r#"{"v": 42}"#).is_err());
}

// ---- vec vec decimal ----

#[test]
fn vec_vec_numbers() {
    #[derive(Deserialize)]
    struct S {
        #[serde(deserialize_with = "crate::serde_util::vec_vec_decimal")]
        v: Vec<Vec<Decimal>>,
    }
    let s: S = serde_json::from_str(r#"{"v": [[1, 2], [3]]}"#).unwrap();
    assert_eq!(s.v.len(), 2);
    assert_eq!(s.v[0], vec![Decimal::from(1), Decimal::from(2)]);
    assert_eq!(s.v[1], vec![Decimal::from(3)]);
}

#[test]
fn vec_vec_empty_outer() {
    #[derive(Deserialize)]
    struct S {
        #[serde(deserialize_with = "crate::serde_util::vec_vec_decimal")]
        v: Vec<Vec<Decimal>>,
    }
    let s: S = serde_json::from_str(r#"{"v": []}"#).unwrap();
    assert!(s.v.is_empty());
}

#[test]
fn vec_vec_empty_inner() {
    #[derive(Deserialize)]
    struct S {
        #[serde(deserialize_with = "crate::serde_util::vec_vec_decimal")]
        v: Vec<Vec<Decimal>>,
    }
    let s: S = serde_json::from_str(r#"{"v": [[], [1]]}"#).unwrap();
    assert_eq!(s.v.len(), 2);
    assert!(s.v[0].is_empty());
}

#[test]
fn vec_vec_rejects_string_in_inner() {
    #[derive(Deserialize)]
    struct S {
        #[serde(deserialize_with = "crate::serde_util::vec_vec_decimal")]
        v: Vec<Vec<Decimal>>,
    }
    assert!(serde_json::from_str::<S>(r#"{"v": [[1, "2"]]}"#).is_err());
}

#[test]
fn vec_vec_rejects_flat_numbers() {
    #[derive(Deserialize)]
    struct S {
        #[serde(deserialize_with = "crate::serde_util::vec_vec_decimal")]
        v: Vec<Vec<Decimal>>,
    }
    assert!(serde_json::from_str::<S>(r#"{"v": [1, 2, 3]}"#).is_err());
}

// ---- untagged enum (the original bug) ----

#[test]
fn untagged_enum_string_not_stolen_by_scalar() {
    /// Mimics TaskOutputOwned with `deserialize_with` on the Decimal variants.
    #[derive(Deserialize, Debug, PartialEq)]
    #[serde(untagged)]
    enum Output {
        Scalar(
            #[serde(deserialize_with = "crate::serde_util::decimal")] Decimal,
        ),
        Vector(
            #[serde(deserialize_with = "crate::serde_util::vec_decimal")]
            Vec<Decimal>,
        ),
        Err(serde_json::Value),
    }

    // Number → Scalar
    let o: Output = serde_json::from_str("42").unwrap();
    assert!(matches!(o, Output::Scalar(d) if d == Decimal::from(42)));

    // Array of numbers → Vector
    let o: Output = serde_json::from_str("[1, 2]").unwrap();
    assert!(matches!(o, Output::Vector(_)));

    // String "94" → Err (NOT Scalar!)
    let o: Output = serde_json::from_str(r#""94""#).unwrap();
    assert!(
        matches!(o, Output::Err(serde_json::Value::String(_))),
        "string \"94\" must fall through to Err, got: {:?}",
        o
    );

    // String "hello" → Err
    let o: Output = serde_json::from_str(r#""hello""#).unwrap();
    assert!(matches!(o, Output::Err(serde_json::Value::String(_))));

    // Null → Err
    let o: Output = serde_json::from_str("null").unwrap();
    assert!(matches!(o, Output::Err(serde_json::Value::Null)));

    // Bool → Err
    let o: Output = serde_json::from_str("true").unwrap();
    assert!(matches!(o, Output::Err(serde_json::Value::Bool(true))));

    // Object → Err
    let o: Output = serde_json::from_str(r#"{"x": 1}"#).unwrap();
    assert!(matches!(o, Output::Err(serde_json::Value::Object(_))));

    // Array of strings → Err (not Vector)
    let o: Output = serde_json::from_str(r#"["a", "b"]"#).unwrap();
    assert!(matches!(o, Output::Err(serde_json::Value::Array(_))));
}

// ---- all fields together ----

#[test]
fn all_fields_happy_path() {
    let json =
        r#"{"bare": 1, "opt": 2.5, "vec": [3, 4], "vecs": [[5], [6, 7]]}"#;
    let a: All = serde_json::from_str(json).unwrap();
    assert_eq!(a.bare, Decimal::from(1));
    assert!(a.opt.is_some());
    assert_eq!(a.vec.len(), 2);
    assert_eq!(a.vecs.len(), 2);
}

#[test]
fn all_fields_opt_null() {
    let json = r#"{"bare": 0, "opt": null, "vec": [], "vecs": []}"#;
    let a: All = serde_json::from_str(json).unwrap();
    assert_eq!(a.opt, None);
}

// ---- arbitrary_precision wire form ----

/// Regression: with serde_json's `arbitrary_precision` on (starlark
/// pulls it in and cargo unifies features workspace-wide), every
/// number reaching `deserialize_any` arrives as a one-entry map keyed
/// by serde_json's private token. Before `NumericDecimalVisitor`
/// grew a `visit_map`, this broke EVERY payload carrying a `Decimal`
/// — including the usage trailer on every agent completion chunk,
/// where the untagged `MessageChunk` masked it as "data did not match
/// any variant".
#[test]
fn decimals_survive_arbitrary_precision() {
    let json = r#"{"bare": 1.5, "opt": 2.5, "vec": [3.25, 4], "vecs": [[5.125], [6, 7]]}"#;
    let a: All = serde_json::from_str(json).unwrap();
    assert_eq!(a.bare, Decimal::try_from(1.5).unwrap());
    assert_eq!(a.opt, Some(Decimal::try_from(2.5).unwrap()));
    assert_eq!(a.vec, vec![Decimal::try_from(3.25).unwrap(), Decimal::from(4)]);
    assert_eq!(a.vecs.len(), 2);
}

/// The literal is parsed EXACTLY under `arbitrary_precision` — no
/// round trip through binary floating point.
#[test]
fn decimal_keeps_full_precision() {
    #[derive(Deserialize)]
    struct S {
        #[serde(deserialize_with = "crate::serde_util::decimal")]
        v: Decimal,
    }
    let s: S = serde_json::from_str(r#"{"v": 0.1234567890123456789}"#).unwrap();
    // f64 cannot hold this; the arbitrary-precision path can.
    assert!(s.v.to_string().starts_with("0.12345678901234567"));
}

/// A map that is NOT serde_json's number token is still a type error,
/// so untagged enums keep falling through to their other variants.
#[test]
fn foreign_map_is_still_rejected() {
    #[derive(Deserialize)]
    struct S {
        #[serde(deserialize_with = "crate::serde_util::decimal")]
        #[allow(dead_code)]
        v: Decimal,
    }
    assert!(serde_json::from_str::<S>(r#"{"v": {"nope": "1"}}"#).is_err());
}
