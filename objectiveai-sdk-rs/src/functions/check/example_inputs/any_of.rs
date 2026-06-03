use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;

use crate::functions::expression::{AnyOfInputSchema, InputValue};

fn max_inner_permutations(schema: &AnyOfInputSchema) -> usize {
    schema
        .any_of
        .iter()
        .map(|v| super::optional::inner_permutations(v))
        .max()
        .unwrap_or(0)
}

pub fn permutations(schema: &AnyOfInputSchema) -> usize {
    schema
        .any_of
        .len()
        .saturating_mul(max_inner_permutations(schema))
}

pub fn generate<R: Rng>(schema: &AnyOfInputSchema, mut rng: R) -> Generator<R> {
    let variant_count = schema.any_of.len();
    let max_inner = max_inner_permutations(schema);

    let generators: Vec<super::multi::Generator> = schema
        .any_of
        .iter()
        .map(|v| {
            super::multi::generate(
                v,
                StdRng::seed_from_u64(rng.random::<u64>()),
            )
        })
        .collect();

    // Each variant appears max_inner times per cycle
    let mut order: Vec<usize> = (0..variant_count)
        .flat_map(|i| std::iter::repeat(i).take(max_inner))
        .collect();
    order.shuffle(&mut rng);

    Generator {
        generators,
        order,
        pos: 0,
        rng,
    }
}

pub struct Generator<R: Rng> {
    generators: Vec<super::multi::Generator>,
    order: Vec<usize>,
    pos: usize,
    rng: R,
}

impl<R: Rng> Iterator for Generator<R> {
    type Item = InputValue;
    fn next(&mut self) -> Option<InputValue> {
        if self.order.is_empty() {
            return None;
        }
        if self.pos >= self.order.len() {
            self.order.shuffle(&mut self.rng);
            self.pos = 0;
        }
        let variant_idx = self.order[self.pos];
        self.pos += 1;
        self.generators[variant_idx].next()
    }
}
