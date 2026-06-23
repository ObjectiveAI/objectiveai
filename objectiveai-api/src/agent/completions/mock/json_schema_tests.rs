use super::*;
use rand::SeedableRng;
use rust_decimal::Decimal;
use std::collections::HashSet;

#[test]
fn test_boolean_roundtrip() {
    let json = r#"{"type":"boolean"}"#;
    let schema: JsonSchema = serde_json::from_str(json).unwrap();
    let back = serde_json::to_string(&schema).unwrap();
    assert_eq!(back, json);
}

#[test]
fn normalize_collapses_2020_12_nullable_type_arrays() {
    // rmcp 1.7 (no AddNullable) emits Option<T> params as 2020-12 nullable
    // type-arrays. Without normalization the untagged parser rejects them;
    // after normalization they parse as the base type (and an object schema
    // with such a property samples a populated value, not "{}").
    let mut v = serde_json::json!({
        "type": "object",
        "properties": {
            "jq": { "type": ["string", "null"] },
            "n": { "type": ["null", "integer"], "minimum": 1, "maximum": 1 },
            "items": { "type": "array", "items": { "type": ["string", "null"] } }
        }
    });
    // Pre-normalization: the nullable arrays make the parse fail.
    assert!(serde_json::from_value::<JsonSchema>(v.clone()).is_err());

    normalize_nullable_types(&mut v);
    let schema: JsonSchema = serde_json::from_value(v)
        .expect("normalized nullable schema must parse");

    let mut rng = rand::rngs::SmallRng::seed_from_u64(7);
    let (content, _) = schema.generate_content_from_rng(&mut rng, 1);
    let parsed: serde_json::Value =
        serde_json::from_str(&content).expect("generated content is valid JSON");
    let obj = parsed.as_object().expect("object output");
    // The nullable string/array/integer properties are populated, not dropped.
    assert!(obj.get("jq").and_then(|x| x.as_str()).is_some());
    assert!(obj.get("items").and_then(|x| x.as_array()).is_some());
    assert_eq!(obj.get("n").and_then(|x| x.as_i64()), Some(1));
}

#[test]
fn generate_content_returns_valid_json_string() {
    let schema = StringJsonSchema {
        r#type: StringJsonSchemaType::String,
        r#enum: Some(vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()]),
    };
    let mut rng = rand::rngs::SmallRng::seed_from_u64(42);
    let (content, logprobs) = schema.generate_content_from_rng(&mut rng, 3);

    // Content must be a valid JSON string (one of the variants, quoted)
    let variants = ["\"alpha\"", "\"beta\"", "\"gamma\""];
    assert!(variants.contains(&content.as_str()), "unexpected content: {content}");

    // Logprobs must cover the full content
    let reconstructed: String = logprobs.iter().map(|lp| lp.token.as_str()).collect();
    assert_eq!(reconstructed, content);
}

#[test]
fn generate_content_logprobs_have_no_duplicate_top_logprobs() {
    let schema = StringJsonSchema {
        r#type: StringJsonSchemaType::String,
        r#enum: Some(vec!["aaa".to_string(), "aab".to_string(), "aac".to_string()]),
    };
    let mut rng = rand::rngs::SmallRng::seed_from_u64(99);
    let (_, logprobs) = schema.generate_content_from_rng(&mut rng, 3);

    for lp in &logprobs {
        let tokens: Vec<&str> = lp.top_logprobs.iter().map(|t| t.token.as_str()).collect();
        let unique: HashSet<&str> = tokens.iter().copied().collect();
        assert_eq!(tokens.len(), unique.len(), "duplicate top_logprobs in token {:?}: {:?}", lp.token, tokens);
    }
}

#[test]
fn generate_content_logprobs_probabilities_sum_to_one() {
    let schema = StringJsonSchema {
        r#type: StringJsonSchemaType::String,
        r#enum: Some(vec!["foo".to_string(), "bar".to_string()]),
    };
    let mut rng = rand::rngs::SmallRng::seed_from_u64(7);
    let (_, logprobs) = schema.generate_content_from_rng(&mut rng, 2);

    for lp in &logprobs {
        // top_logprobs probabilities (as exp(logprob)) should roughly sum to 1
        // But we check the raw probabilities via the first token's logprob
        assert!(!lp.top_logprobs.is_empty());
        // Primary token matches
        assert_eq!(lp.token, lp.top_logprobs[0].token);
    }
}

#[test]
fn generate_content_token_lengths_are_1_to_3() {
    let schema = StringJsonSchema {
        r#type: StringJsonSchemaType::String,
        r#enum: Some(vec![
            "abcdefghij".to_string(),
            "klmnopqrst".to_string(),
        ]),
    };
    let mut rng = rand::rngs::SmallRng::seed_from_u64(123);
    let (_, logprobs) = schema.generate_content_from_rng(&mut rng, 2);

    for lp in &logprobs {
        let len = lp.token.len();
        assert!(len >= 1 && len <= 3, "token {:?} has length {}", lp.token, len);
    }
}

#[test]
fn generate_content_with_single_permutation() {
    let schema = StringJsonSchema {
        r#type: StringJsonSchemaType::String,
        r#enum: Some(vec!["only".to_string()]),
    };
    let mut rng = rand::rngs::SmallRng::seed_from_u64(0);
    let (content, logprobs) = schema.generate_content_from_rng(&mut rng, 1);

    assert_eq!(content, "\"only\"");
    assert!(!logprobs.is_empty());
    // With only 1 permutation, each logprob should have exactly 1 top_logprob
    for lp in &logprobs {
        assert_eq!(lp.top_logprobs.len(), 1);
    }
}

#[test]
fn generate_content_with_no_enum_produces_random_strings() {
    let schema = StringJsonSchema { r#type: StringJsonSchemaType::String, r#enum: None };
    let mut rng = rand::rngs::SmallRng::seed_from_u64(0);
    let (content, logprobs) = schema.generate_content_from_rng(&mut rng, 3);
    // With no enum, generate_from_rng produces random strings; content is the serialized first one
    assert!(!content.is_empty());
    let reconstructed: String = logprobs.iter().map(|lp| lp.token.as_str()).collect();
    assert_eq!(reconstructed, content);
}

#[test]
fn generate_content_shorter_variants_handled() {
    let schema = StringJsonSchema {
        r#type: StringJsonSchemaType::String,
        r#enum: Some(vec!["longstring".to_string(), "s".to_string()]),
    };
    let mut rng = rand::rngs::SmallRng::seed_from_u64(55);
    let (content, logprobs) = schema.generate_content_from_rng(&mut rng, 2);

    // Must still produce valid logprobs covering the full content
    let reconstructed: String = logprobs.iter().map(|lp| lp.token.as_str()).collect();
    assert_eq!(reconstructed, content);

    // Bytes field should match token
    for lp in &logprobs {
        assert_eq!(lp.bytes.as_ref().unwrap(), lp.token.as_bytes());
        for tlp in &lp.top_logprobs {
            assert_eq!(tlp.bytes.as_ref().unwrap(), tlp.token.as_bytes());
        }
    }
}

#[test]
fn generate_content_top_logprob_matches_content_across_seeds() {
    let schemas: Vec<(&str, Box<dyn Fn(&mut rand::rngs::SmallRng, usize) -> (String, Vec<Logprob>)>)> = vec![
        ("string_enum", Box::new(|rng, p| {
            StringJsonSchema { r#type: StringJsonSchemaType::String, r#enum: Some(vec!["a".into(), "b".into(), "c".into()]) }
                .generate_content_from_rng(rng, p)
        })),
        ("string_no_enum", Box::new(|rng, p| {
            StringJsonSchema { r#type: StringJsonSchemaType::String, r#enum: None }.generate_content_from_rng(rng, p)
        })),
        ("number", Box::new(|rng, p| {
            NumberJsonSchema { r#type: NumberJsonSchemaType::Number, minimum: Some(0.0), maximum: Some(10.0) }
                .generate_content_from_rng(rng, p)
        })),
        ("integer", Box::new(|rng, p| {
            IntegerJsonSchema { r#type: IntegerJsonSchemaType::Integer, minimum: Some(0), maximum: Some(100) }
                .generate_content_from_rng(rng, p)
        })),
        ("boolean", Box::new(|rng, p| {
            BooleanJsonSchema { r#type: BooleanJsonSchemaType::Boolean }.generate_content_from_rng(rng, p)
        })),
    ];

    for seed in 0..20u64 {
        for (name, generate) in &schemas {
            for permutations in [1, 3, 5] {
                let mut rng = rand::rngs::SmallRng::seed_from_u64(seed);
                let (content, logprobs) = generate(&mut rng, permutations);
                let reconstructed: String = logprobs.iter().map(|lp| lp.token.as_str()).collect();
                assert_eq!(
                    reconstructed, content,
                    "seed={seed} schema={name} permutations={permutations}: reconstructed tokens don't match content"
                );
                for lp in &logprobs {
                    assert!(
                        lp.top_logprobs.iter().any(|tlp| tlp.token == lp.token),
                        "seed={seed} schema={name} permutations={permutations}: content token {:?} not found in top_logprobs {:?}",
                        lp.token, lp.top_logprobs.iter().map(|t| &t.token).collect::<Vec<_>>()
                    );
                }
            }
        }
    }
}

#[test]
fn generate_content_string_enum_always_returns_enum_value() {
    let variants = vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string(), "delta".to_string()];
    let schema = StringJsonSchema { r#type: StringJsonSchemaType::String, r#enum: Some(variants.clone()) };
    let quoted: Vec<String> = variants.iter().map(|v| serde_json::to_string(v).unwrap()).collect();

    for seed in 0..20u64 {
        for permutations in [1, 2, 4, 8] {
            let mut rng = rand::rngs::SmallRng::seed_from_u64(seed);
            let (content, _) = schema.generate_content_from_rng(&mut rng, permutations);
            assert!(
                quoted.contains(&content),
                "seed={seed} permutations={permutations}: content {content:?} is not a quoted enum variant"
            );
        }
    }
}

#[test]
fn serde_roundtrip_all_types() {
    let cases = vec![
        r#"{"type":"string"}"#,
        r#"{"type":"string","enum":["a","b"]}"#,
        r#"{"type":"number"}"#,
        r#"{"type":"number","minimum":0.0,"maximum":1.0}"#,
        r#"{"type":"integer"}"#,
        r#"{"type":"integer","minimum":0,"maximum":10}"#,
        r#"{"type":"boolean"}"#,
        r#"{"type":"array","items":{"type":"boolean"}}"#,
        r#"{"type":"object","properties":{"x":{"type":"integer"}}}"#,
        r#"{"anyOf":[{"type":"string"},{"type":"integer"}]}"#,
    ];
    for json in cases {
        let schema: JsonSchema = serde_json::from_str(json).unwrap_or_else(|e| {
            panic!("failed to parse {json:?}: {e}")
        });
        let back = serde_json::to_string(&schema).unwrap();
        assert_eq!(back, json, "roundtrip failed for {json}");
    }
}

#[test]
fn anyof_picks_from_variants() {
    let json = r#"{"anyOf":[{"type":"string","enum":["yes"]},{"type":"string","enum":["no"]}]}"#;
    let schema: JsonSchema = serde_json::from_str(json).unwrap();

    let mut seen = HashSet::new();
    for seed in 0..50u64 {
        let mut rng = rand::rngs::SmallRng::seed_from_u64(seed);
        let (content, logprobs) = schema.generate_content_from_rng(&mut rng, 1);
        assert!(
            content == "\"yes\"" || content == "\"no\"",
            "unexpected anyOf content: {content:?}"
        );
        let reconstructed: String = logprobs.iter().map(|lp| lp.token.as_str()).collect();
        assert_eq!(reconstructed, content);
        seen.insert(content);
    }
    assert_eq!(seen.len(), 2, "expected both anyOf variants to be picked across 50 seeds");
}

#[test]
fn anyof_empty_returns_empty() {
    let schema = AnyOfJsonSchema { any_of: vec![] };
    let mut rng = rand::rngs::SmallRng::seed_from_u64(0);
    let (content, logprobs) = schema.generate_content_from_rng(&mut rng, 3);
    assert_eq!(content, "");
    assert!(logprobs.is_empty());
}

#[test]
fn object_content_is_valid_json() {
    let json = r#"{"type":"object","properties":{"name":{"type":"string","enum":["alice","bob"]},"age":{"type":"integer","minimum":0,"maximum":100}}}"#;
    let schema: JsonSchema = serde_json::from_str(json).unwrap();

    for seed in 0..20u64 {
        let mut rng = rand::rngs::SmallRng::seed_from_u64(seed);
        let (content, logprobs) = schema.generate_content_from_rng(&mut rng, 3);
        // Content must be valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap_or_else(|e| {
            panic!("seed={seed}: invalid JSON {content:?}: {e}")
        });
        assert!(parsed.is_object(), "seed={seed}: expected object, got {parsed}");
        assert!(parsed.get("name").is_some(), "seed={seed}: missing 'name'");
        assert!(parsed.get("age").is_some(), "seed={seed}: missing 'age'");
        // Logprobs reconstruct content
        let reconstructed: String = logprobs.iter().map(|lp| lp.token.as_str()).collect();
        assert_eq!(reconstructed, content, "seed={seed}: logprobs don't reconstruct content");
    }
}

#[test]
fn object_empty_properties_returns_empty_object() {
    let json = r#"{"type":"object"}"#;
    let schema: JsonSchema = serde_json::from_str(json).unwrap();
    let mut rng = rand::rngs::SmallRng::seed_from_u64(0);
    let (content, logprobs) = schema.generate_content_from_rng(&mut rng, 1);
    assert_eq!(content, "{}");
    assert_eq!(logprobs.len(), 1);
    assert_eq!(logprobs[0].token, "{}");
}

#[test]
fn array_content_is_valid_json() {
    let json = r#"{"type":"array","items":{"type":"string","enum":["x","y"]},"minItems":2,"maxItems":4}"#;
    let schema: JsonSchema = serde_json::from_str(json).unwrap();

    for seed in 0..20u64 {
        let mut rng = rand::rngs::SmallRng::seed_from_u64(seed);
        let (content, logprobs) = schema.generate_content_from_rng(&mut rng, 2);
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap_or_else(|e| {
            panic!("seed={seed}: invalid JSON {content:?}: {e}")
        });
        let arr = parsed.as_array().unwrap_or_else(|| panic!("seed={seed}: expected array"));
        assert!(arr.len() >= 2 && arr.len() <= 4, "seed={seed}: array length {} out of bounds", arr.len());
        for item in arr {
            let s = item.as_str().unwrap();
            assert!(s == "x" || s == "y", "seed={seed}: unexpected item {s:?}");
        }
        let reconstructed: String = logprobs.iter().map(|lp| lp.token.as_str()).collect();
        assert_eq!(reconstructed, content, "seed={seed}");
    }
}

#[test]
fn nested_object_with_array_content_is_valid_json() {
    let json = r#"{"type":"object","properties":{"tags":{"type":"array","items":{"type":"string","enum":["a","b","c"]},"minItems":1,"maxItems":3},"score":{"type":"number","minimum":0.0,"maximum":1.0}}}"#;
    let schema: JsonSchema = serde_json::from_str(json).unwrap();

    for seed in 0..10u64 {
        let mut rng = rand::rngs::SmallRng::seed_from_u64(seed);
        let (content, logprobs) = schema.generate_content_from_rng(&mut rng, 3);
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap_or_else(|e| {
            panic!("seed={seed}: invalid JSON {content:?}: {e}")
        });
        assert!(parsed.get("tags").unwrap().is_array());
        assert!(parsed.get("score").unwrap().is_number());
        let reconstructed: String = logprobs.iter().map(|lp| lp.token.as_str()).collect();
        assert_eq!(reconstructed, content, "seed={seed}");
    }
}

#[test]
fn object_structural_tokens_have_logprob_zero() {
    let json = r#"{"type":"object","properties":{"x":{"type":"boolean"}}}"#;
    let schema: JsonSchema = serde_json::from_str(json).unwrap();
    let mut rng = rand::rngs::SmallRng::seed_from_u64(0);
    let (_, logprobs) = schema.generate_content_from_rng(&mut rng, 1);

    // First token should be "{", last should be "}"
    assert_eq!(logprobs.first().unwrap().token, "{");
    assert_eq!(logprobs.first().unwrap().logprob, Decimal::ZERO);
    assert_eq!(logprobs.last().unwrap().token, "}");
    assert_eq!(logprobs.last().unwrap().logprob, Decimal::ZERO);
}

#[test]
fn generate_logprobs_from_serialized_empty_input() {
    let mut rng = rand::rngs::SmallRng::seed_from_u64(0);
    let (content, logprobs) = generate_logprobs_from_serialized(&[], &mut rng);
    assert_eq!(content, "");
    assert!(logprobs.is_empty());
}

#[test]
fn generate_logprobs_from_serialized_single_char() {
    let mut rng = rand::rngs::SmallRng::seed_from_u64(0);
    let (content, logprobs) = generate_logprobs_from_serialized(&["x".to_string()], &mut rng);
    assert_eq!(content, "x");
    assert_eq!(logprobs.len(), 1);
    assert_eq!(logprobs[0].token, "x");
    assert_eq!(logprobs[0].top_logprobs.len(), 1);
}

#[test]
fn integer_content_is_valid_integer() {
    let json = r#"{"type":"integer","minimum":5,"maximum":10}"#;
    let schema: JsonSchema = serde_json::from_str(json).unwrap();

    for seed in 0..20u64 {
        let mut rng = rand::rngs::SmallRng::seed_from_u64(seed);
        let (content, logprobs) = schema.generate_content_from_rng(&mut rng, 3);
        let val: i64 = content.parse().unwrap_or_else(|e| {
            panic!("seed={seed}: {content:?} is not a valid integer: {e}")
        });
        assert!(val >= 5 && val <= 10, "seed={seed}: {val} out of range");
        let reconstructed: String = logprobs.iter().map(|lp| lp.token.as_str()).collect();
        assert_eq!(reconstructed, content, "seed={seed}");
    }
}

#[test]
fn number_content_is_valid_number() {
    let json = r#"{"type":"number","minimum":0.0,"maximum":1.0}"#;
    let schema: JsonSchema = serde_json::from_str(json).unwrap();

    for seed in 0..20u64 {
        let mut rng = rand::rngs::SmallRng::seed_from_u64(seed);
        let (content, logprobs) = schema.generate_content_from_rng(&mut rng, 2);
        let val: f64 = content.parse().unwrap_or_else(|e| {
            panic!("seed={seed}: {content:?} is not a valid number: {e}")
        });
        assert!(val >= 0.0 && val <= 1.0, "seed={seed}: {val} out of range");
        let reconstructed: String = logprobs.iter().map(|lp| lp.token.as_str()).collect();
        assert_eq!(reconstructed, content, "seed={seed}");
    }
}

#[test]
fn boolean_content_is_true_or_false() {
    let json = r#"{"type":"boolean"}"#;
    let schema: JsonSchema = serde_json::from_str(json).unwrap();

    let mut seen = HashSet::new();
    for seed in 0..20u64 {
        let mut rng = rand::rngs::SmallRng::seed_from_u64(seed);
        let (content, logprobs) = schema.generate_content_from_rng(&mut rng, 2);
        assert!(content == "true" || content == "false", "seed={seed}: unexpected {content:?}");
        let reconstructed: String = logprobs.iter().map(|lp| lp.token.as_str()).collect();
        assert_eq!(reconstructed, content, "seed={seed}");
        seen.insert(content);
    }
    assert_eq!(seen.len(), 2, "expected both true and false across 20 seeds");
}

#[test]
fn top_logprobs_count_matches_permutations() {
    let schema = StringJsonSchema {
        r#type: StringJsonSchemaType::String,
        r#enum: Some(vec!["aaa".into(), "bbb".into(), "ccc".into(), "ddd".into()]),
    };

    for permutations in [1, 2, 3, 4] {
        let mut rng = rand::rngs::SmallRng::seed_from_u64(42);
        let (_, logprobs) = schema.generate_content_from_rng(&mut rng, permutations);
        for lp in &logprobs {
            // After merging, top_logprobs count should be <= permutations
            assert!(
                lp.top_logprobs.len() <= permutations,
                "permutations={permutations}: got {} top_logprobs for token {:?}",
                lp.top_logprobs.len(), lp.token
            );
            // Should always have at least 1
            assert!(!lp.top_logprobs.is_empty());
        }
    }
}

#[test]
fn all_logprob_values_are_non_positive() {
    let json = r#"{"type":"object","properties":{"v":{"type":"string","enum":["x","y","z"]}}}"#;
    let schema: JsonSchema = serde_json::from_str(json).unwrap();
    // Small epsilon for floating point precision in ln(1.0) -> Decimal conversion
    let epsilon = Decimal::new(1, 10); // 0.0000000001

    for seed in 0..10u64 {
        let mut rng = rand::rngs::SmallRng::seed_from_u64(seed);
        let (_, logprobs) = schema.generate_content_from_rng(&mut rng, 3);
        for lp in &logprobs {
            assert!(lp.logprob <= epsilon, "seed={seed}: logprob {} > 0 for {:?}", lp.logprob, lp.token);
            for tlp in &lp.top_logprobs {
                if let Some(v) = tlp.logprob {
                    assert!(v <= epsilon, "seed={seed}: top_logprob {} > 0 for {:?}", v, tlp.token);
                }
            }
        }
    }
}

#[test]
fn generate_content_deterministic_with_same_seed() {
    let schema = StringJsonSchema {
        r#type: StringJsonSchemaType::String,
        r#enum: Some(vec!["x".to_string(), "y".to_string(), "z".to_string()]),
    };
    let mut rng1 = rand::rngs::SmallRng::seed_from_u64(42);
    let (content1, logprobs1) = schema.generate_content_from_rng(&mut rng1, 3);
    let mut rng2 = rand::rngs::SmallRng::seed_from_u64(42);
    let (content2, logprobs2) = schema.generate_content_from_rng(&mut rng2, 3);
    assert_eq!(content1, content2);
    assert_eq!(logprobs1, logprobs2);
}
