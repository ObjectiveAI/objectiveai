use rand::rngs::StdRng;

use crate::functions::expression::{InputSchema, InputValue};

pub fn generate(schema: &InputSchema, rng: StdRng) -> Generator {
    match schema {
        InputSchema::Boolean(s) => {
            Generator::Boolean(super::boolean::generate(s, rng))
        }
        InputSchema::String(s) => {
            Generator::String(super::string::generate(s, rng))
        }
        InputSchema::Integer(s) => {
            Generator::Integer(super::integer::generate(s, rng))
        }
        InputSchema::Number(s) => {
            Generator::Number(super::number::generate(s, rng))
        }
        InputSchema::Image(s) => {
            Generator::Image(super::image::generate(s, rng))
        }
        InputSchema::Audio(s) => {
            Generator::Audio(super::audio::generate(s, rng))
        }
        InputSchema::Video(s) => {
            Generator::Video(super::video::generate(s, rng))
        }
        InputSchema::File(s) => Generator::File(super::file::generate(s, rng)),
        InputSchema::Object(s) => {
            Generator::Object(super::object::generate(s, rng))
        }
        InputSchema::Array(s) => {
            Generator::Array(super::array::generate(s, rng))
        }
        InputSchema::AnyOf(s) => {
            Generator::AnyOf(super::any_of::generate(s, rng))
        }
    }
}

pub enum Generator {
    Boolean(super::boolean::Generator<StdRng>),
    String(super::string::Generator<StdRng>),
    Integer(super::integer::Generator<StdRng>),
    Number(super::number::Generator<StdRng>),
    Image(super::image::Generator<StdRng>),
    Audio(super::audio::Generator<StdRng>),
    Video(super::video::Generator<StdRng>),
    File(super::file::Generator<StdRng>),
    Object(super::object::Generator),
    Array(super::array::Generator<StdRng>),
    AnyOf(super::any_of::Generator<StdRng>),
}

impl Iterator for Generator {
    type Item = InputValue;
    fn next(&mut self) -> Option<InputValue> {
        match self {
            Generator::Boolean(g) => g.next(),
            Generator::String(g) => g.next(),
            Generator::Integer(g) => g.next(),
            Generator::Number(g) => g.next(),
            Generator::Image(g) => g.next(),
            Generator::Audio(g) => g.next(),
            Generator::Video(g) => g.next(),
            Generator::File(g) => g.next(),
            Generator::Object(g) => g.next(),
            Generator::Array(g) => g.next(),
            Generator::AnyOf(g) => g.next(),
        }
    }
}
