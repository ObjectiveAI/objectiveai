use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;

use crate::functions::expression::{ArrayInputSchema, InputValue};

fn length_range(schema: &ArrayInputSchema) -> (usize, usize) {
    match (schema.min_items, schema.max_items) {
        (Some(min), Some(max)) => (min as usize, max as usize),
        (Some(min), None) => (min as usize, min as usize + 10),
        (None, Some(max)) => ((max as usize).saturating_sub(10), max as usize),
        (None, None) => (0, 10),
    }
}

/// Compute the distinct lengths: min, mid=(min+max)/2, max — deduplicated.
fn distinct_lengths(schema: &ArrayInputSchema) -> Vec<usize> {
    let (min, max) = length_range(schema);
    let mid = (min + max) / 2;
    let mut lengths = vec![min];
    if mid != min {
        lengths.push(mid);
    }
    if max != min && max != mid {
        lengths.push(max);
    }
    lengths
}

pub fn permutations(schema: &ArrayInputSchema) -> usize {
    let item_perms = super::optional::inner_permutations(schema.items.as_ref());
    item_perms.saturating_mul(distinct_lengths(schema).len())
}

pub fn generate<R: Rng>(schema: &ArrayInputSchema, mut rng: R) -> Generator<R> {
    let item_schema = schema.items.as_ref().clone();
    let lengths = distinct_lengths(schema);

    // For each distinct length, create independent generators per index.
    let per_length: Vec<Vec<Box<super::multi::Generator>>> = lengths
        .iter()
        .map(|&len| {
            (0..len)
                .map(|_| {
                    Box::new(super::multi::generate(
                        &item_schema,
                        StdRng::seed_from_u64(rng.random::<u64>()),
                    ))
                })
                .collect()
        })
        .collect();

    let mut length_order: Vec<usize> = (0..lengths.len()).collect();
    length_order.shuffle(&mut rng);

    Generator {
        per_length,
        length_order,
        pos: 0,
        rng,
    }
}

pub struct Generator<R: Rng> {
    /// For each length option, a vec of generators (one per array index).
    per_length: Vec<Vec<Box<super::multi::Generator>>>,
    length_order: Vec<usize>,
    pos: usize,
    rng: R,
}

impl<R: Rng> Iterator for Generator<R> {
    type Item = InputValue;
    fn next(&mut self) -> Option<InputValue> {
        // Round-robin through shuffled length order, reshuffle each cycle
        let li = self.length_order[self.pos];
        self.pos += 1;
        if self.pos >= self.length_order.len() {
            self.pos = 0;
            self.length_order.shuffle(&mut self.rng);
        }

        // Build array: each index has its own generator for this length
        let arr: Vec<InputValue> = self.per_length[li]
            .iter_mut()
            .map(|g| g.next().unwrap())
            .collect();

        Some(InputValue::Array(arr))
    }
}
