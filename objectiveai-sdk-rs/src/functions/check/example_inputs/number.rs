use rand::Rng;
use rand::seq::SliceRandom;

use crate::functions::expression::{InputValue, NumberInputSchema};

pub fn permutations(schema: &NumberInputSchema) -> usize {
    let min = schema.minimum;
    let max = schema.maximum;
    let mut count = 0;

    if min.map_or(true, |m| m <= 0.0) && max.map_or(true, |m| m >= 0.0) {
        count += 1;
    }
    if let Some(m) = min {
        if m != 0.0 {
            count += 1;
        }
    }
    if let Some(m) = max {
        if m != 0.0 {
            count += 1;
        }
    }
    if min.is_none() {
        count += 1;
    }
    if max.is_none() {
        count += 1;
    }
    if min.map_or(true, |m| m < 0.0) && max.map_or(true, |m| m >= 0.0) {
        count += 1;
    }
    if min.map_or(true, |m| m <= 0.0) && max.map_or(true, |m| m > 0.0) {
        count += 1;
    }

    count.max(1)
}

#[derive(Clone, Copy)]
enum Variant {
    Zero,
    Min(f64),
    Max(f64),
    RandomNegative,
    RandomPositive,
    DecimalNeg,
    DecimalPos,
}

fn variants(schema: &NumberInputSchema) -> Vec<Variant> {
    let min = schema.minimum;
    let max = schema.maximum;
    let mut v = Vec::with_capacity(7);

    if min.map_or(true, |m| m <= 0.0) && max.map_or(true, |m| m >= 0.0) {
        v.push(Variant::Zero);
    }
    if let Some(m) = min {
        if m != 0.0 {
            v.push(Variant::Min(m));
        }
    }
    if let Some(m) = max {
        if m != 0.0 {
            v.push(Variant::Max(m));
        }
    }
    if min.is_none() {
        v.push(Variant::RandomNegative);
    }
    if max.is_none() {
        v.push(Variant::RandomPositive);
    }
    if min.map_or(true, |m| m < 0.0) && max.map_or(true, |m| m >= 0.0) {
        v.push(Variant::DecimalNeg);
    }
    if min.map_or(true, |m| m <= 0.0) && max.map_or(true, |m| m > 0.0) {
        v.push(Variant::DecimalPos);
    }
    if v.is_empty() {
        v.push(Variant::Zero);
    }
    v
}

pub fn generate<R: Rng>(
    schema: &NumberInputSchema,
    mut rng: R,
) -> Generator<R> {
    let vars = variants(schema);
    let mut indices: Vec<usize> = (0..vars.len()).collect();
    indices.shuffle(&mut rng);
    Generator {
        variants: vars,
        indices,
        pos: 0,
        rng,
        min: schema.minimum,
        max: schema.maximum,
    }
}

pub struct Generator<R: Rng> {
    variants: Vec<Variant>,
    indices: Vec<usize>,
    pos: usize,
    rng: R,
    min: Option<f64>,
    max: Option<f64>,
}

impl<R: Rng> Iterator for Generator<R> {
    type Item = InputValue;
    fn next(&mut self) -> Option<InputValue> {
        if self.pos >= self.indices.len() {
            self.indices.shuffle(&mut self.rng);
            self.pos = 0;
        }
        let index = self.indices[self.pos];
        self.pos += 1;
        let value = match self.variants[index] {
            Variant::Zero => 0.0,
            Variant::Min(m) => m,
            Variant::Max(m) => m,
            Variant::RandomNegative => {
                let upper = self.max.unwrap_or(-1.0).min(-1.0);
                self.rng.random_range(-100.0..=upper)
            }
            Variant::RandomPositive => {
                let lower = self.min.unwrap_or(1.0).max(1.0);
                self.rng.random_range(lower..=100.0)
            }
            Variant::DecimalNeg => self.rng.random_range(-1.0..0.0),
            Variant::DecimalPos => self.rng.random_range(0.0..1.0),
        };
        Some(InputValue::Number(value))
    }
}
