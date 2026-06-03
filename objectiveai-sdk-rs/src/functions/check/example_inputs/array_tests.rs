use std::collections::{HashMap, HashSet};

use crate::functions::check::example_inputs::array;
use crate::functions::expression::{
    ArrayInputSchema, BooleanInputSchema, InputSchema, InputValue,
    StringInputSchema,
};

/// Collect all outputs from the array generator and verify:
/// 1. Every unique length seen has every index covered with all item permutations.
/// 2. The total number of items matches `permutations`.
fn assert_per_index_coverage(schema: &ArrayInputSchema) {
    let item_perms = super::optional::inner_permutations(schema.items.as_ref());
    let total = array::permutations(schema);

    let rng = rand::rng();
    let iter = array::generate(schema, rng);

    // length -> index -> set of observed items
    let mut seen: HashMap<usize, HashMap<usize, HashSet<InputValue>>> =
        HashMap::new();
    let mut count = 0usize;

    for item in iter.take(total) {
        count += 1;
        let arr = match item {
            InputValue::Array(a) => a,
            other => panic!("expected Array, got {other:?}"),
        };
        let len = arr.len();
        let by_index = seen.entry(len).or_default();
        for (idx, val) in arr.into_iter().enumerate() {
            by_index.entry(idx).or_default().insert(val);
        }
    }

    assert_eq!(
        count, total,
        "generator yielded {count} items, expected {total}"
    );

    let num_lengths = seen.len();

    // For each unique length, every index must see all item permutations.
    for (&len, by_index) in &seen {
        assert_eq!(
            by_index.len(),
            len,
            "length {len}: expected {len} index entries, got {}",
            by_index.len(),
        );
        for idx in 0..len {
            let unique = by_index.get(&idx).map_or(0, |s| s.len());
            assert_eq!(
                unique, item_perms,
                "length {len}, index {idx}: expected {item_perms} unique items, got {unique}",
            );
        }
    }

    // Permutations = item_perms * num_lengths
    assert_eq!(
        total,
        item_perms * num_lengths,
        "total permutations mismatch: {total} != {item_perms} * {num_lengths}",
    );
}

// --- Boolean items (2 item permutations) ---

#[test]
fn bool_array_three_lengths() {
    // min=2, max=6 → distinct lengths: 2, 4, 6 → multiplier 3
    // item_perms = 2, total = 6
    let schema = ArrayInputSchema {
        r#type: Default::default(),
        description: None,
        items: Box::new(InputSchema::Boolean(BooleanInputSchema {
            r#type: Default::default(),
            description: None,
        })),
        min_items: Some(2),
        max_items: Some(6),
    };
    assert_per_index_coverage(&schema);
}

#[test]
fn bool_array_two_lengths() {
    // min=3, max=4 → mid=3 (dedup with min) → distinct lengths: 3, 4 → multiplier 2
    // item_perms = 2, total = 4
    let schema = ArrayInputSchema {
        r#type: Default::default(),
        description: None,
        items: Box::new(InputSchema::Boolean(BooleanInputSchema {
            r#type: Default::default(),
            description: None,
        })),
        min_items: Some(3),
        max_items: Some(4),
    };
    assert_per_index_coverage(&schema);
}

#[test]
fn bool_array_one_length() {
    // min=5, max=5 → distinct lengths: 5 → multiplier 1
    // item_perms = 2, total = 2
    let schema = ArrayInputSchema {
        r#type: Default::default(),
        description: None,
        items: Box::new(InputSchema::Boolean(BooleanInputSchema {
            r#type: Default::default(),
            description: None,
        })),
        min_items: Some(5),
        max_items: Some(5),
    };
    assert_per_index_coverage(&schema);
}

// --- String enum items (3 item permutations) ---

#[test]
fn enum_array_three_lengths() {
    // min=1, max=5 → distinct lengths: 1, 3, 5 → multiplier 3
    // item_perms = 3, total = 9
    let schema = ArrayInputSchema {
        r#type: Default::default(),
        description: None,
        items: Box::new(InputSchema::String(StringInputSchema {
            r#type: Default::default(),
            description: None,
            r#enum: Some(vec!["a".into(), "b".into(), "c".into()]),
        })),
        min_items: Some(1),
        max_items: Some(5),
    };
    assert_per_index_coverage(&schema);
}

#[test]
fn enum_array_one_length() {
    // min=2, max=2 → distinct lengths: 2 → multiplier 1
    // item_perms = 3, total = 3
    let schema = ArrayInputSchema {
        r#type: Default::default(),
        description: None,
        items: Box::new(InputSchema::String(StringInputSchema {
            r#type: Default::default(),
            description: None,
            r#enum: Some(vec!["x".into(), "y".into(), "z".into()]),
        })),
        min_items: Some(2),
        max_items: Some(2),
    };
    assert_per_index_coverage(&schema);
}

// --- Plain string items (2 item permutations, random strings) ---

#[test]
fn plain_string_array_two_lengths() {
    // min=0, max=1 → mid=0 (dedup with min) → distinct lengths: 0, 1 → multiplier 2
    // item_perms = 2, total = 4
    // Length 0 has no indices to check, length 1 has index 0.
    let schema = ArrayInputSchema {
        r#type: Default::default(),
        description: None,
        items: Box::new(InputSchema::String(StringInputSchema {
            r#type: Default::default(),
            description: None,
            r#enum: None,
        })),
        min_items: Some(0),
        max_items: Some(1),
    };
    assert_per_index_coverage(&schema);
}

// --- Defaults (no min/max) ---

#[test]
fn defaults_no_bounds() {
    // (None, None) → range (0, 10) → lengths: 0, 5, 10 → multiplier 3
    // item_perms = 2, total = 6
    let schema = ArrayInputSchema {
        r#type: Default::default(),
        description: None,
        items: Box::new(InputSchema::Boolean(BooleanInputSchema {
            r#type: Default::default(),
            description: None,
        })),
        min_items: None,
        max_items: None,
    };
    assert_per_index_coverage(&schema);
}

#[test]
fn defaults_only_min() {
    // (Some(3), None) → range (3, 13) → lengths: 3, 8, 13 → multiplier 3
    // item_perms = 2, total = 6
    let schema = ArrayInputSchema {
        r#type: Default::default(),
        description: None,
        items: Box::new(InputSchema::Boolean(BooleanInputSchema {
            r#type: Default::default(),
            description: None,
        })),
        min_items: Some(3),
        max_items: None,
    };
    assert_per_index_coverage(&schema);
}

#[test]
fn defaults_only_max() {
    // (None, Some(8)) → range (0, 8) → lengths: 0, 4, 8 → multiplier 3
    // item_perms = 2, total = 6
    let schema = ArrayInputSchema {
        r#type: Default::default(),
        description: None,
        items: Box::new(InputSchema::Boolean(BooleanInputSchema {
            r#type: Default::default(),
            description: None,
        })),
        min_items: None,
        max_items: Some(8),
    };
    assert_per_index_coverage(&schema);
}

// --- Higher item permutations via nested schema ---

#[test]
fn enum_4_variants_three_lengths() {
    // min=1, max=3 → lengths: 1, 2, 3 → multiplier 3
    // item_perms = 4, total = 12
    let schema = ArrayInputSchema {
        r#type: Default::default(),
        description: None,
        items: Box::new(InputSchema::String(StringInputSchema {
            r#type: Default::default(),
            description: None,
            r#enum: Some(vec!["w".into(), "x".into(), "y".into(), "z".into()]),
        })),
        min_items: Some(1),
        max_items: Some(3),
    };
    assert_per_index_coverage(&schema);
}
