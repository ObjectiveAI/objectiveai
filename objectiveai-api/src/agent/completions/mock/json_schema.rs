use indexmap::IndexMap;
use objectiveai_sdk::agent::completions::response::{Logprob, TopLogprob};
use rand::Rng;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum JsonSchema {
    String(StringJsonSchema),
    Number(NumberJsonSchema),
    Integer(IntegerJsonSchema),
    Boolean(BooleanJsonSchema),
    Array(ArrayJsonSchema),
    Object(ObjectJsonSchema),
    AnyOf(AnyOfJsonSchema),
}

/// Collapse JSON-Schema-2020-12 nullable type arrays into the scalar base
/// type this module's `#[serde(untagged)]` parser understands.
///
/// rmcp 1.7 dropped the `AddNullable` schema transform, so an `Option<T>`
/// tool/response parameter now serialises as `{"type":["string","null"]}`
/// (2020-12 nullable) instead of the pre-1.7 `{"type":"string","nullable":true}`.
/// The leaf variants here key off a *scalar* `type`, so a `type` array matches
/// no variant and the whole parse fails (yielding empty `"{}"` output).
///
/// Rule: for any object with an array-valued `"type"`, replace it with the
/// first non-`"null"` entry — i.e. treat a nullable field as its (non-null)
/// base type, exactly as the pre-1.7 `nullable:true` form behaved here (the
/// mock ignored `nullable` and always generated the value). Recurses through
/// `properties`, `items`, `anyOf`, and every other nested value.
pub(super) fn normalize_nullable_types(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(serde_json::Value::Array(types)) = map.get_mut("type") {
                if let Some(base) =
                    types.iter().find(|t| t.as_str() != Some("null")).cloned()
                {
                    map.insert("type".into(), base);
                }
            }
            for v in map.values_mut() {
                normalize_nullable_types(v);
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr.iter_mut() {
                normalize_nullable_types(v);
            }
        }
        _ => {}
    }
}

impl JsonSchema {
    pub fn generate_content(&self, permutations: usize) -> (String, Vec<Logprob>) {
        self.generate_content_from_rng(&mut rand::rng(), permutations)
    }

    pub fn generate_content_from_rng(
        &self,
        rng: &mut impl Rng,
        permutations: usize,
    ) -> (String, Vec<Logprob>) {
        match self {
            JsonSchema::String(s) => s.generate_content_from_rng(rng, permutations),
            JsonSchema::Number(n) => n.generate_content_from_rng(rng, permutations),
            JsonSchema::Integer(i) => i.generate_content_from_rng(rng, permutations),
            JsonSchema::Boolean(b) => b.generate_content_from_rng(rng, permutations),
            JsonSchema::Array(a) => a.generate_content_from_rng(rng, permutations),
            JsonSchema::Object(o) => o.generate_content_from_rng(rng, permutations),
            JsonSchema::AnyOf(a) => a.generate_content_from_rng(rng, permutations),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum StringJsonSchemaType {
    String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StringJsonSchema {
    r#type: StringJsonSchemaType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#enum: Option<Vec<String>>,
}

impl StringJsonSchema {
    fn generate_value_from_rng(&self, rng: &mut impl Rng) -> serde_json::Value {
        let s = match &self.r#enum {
            Some(variants) if !variants.is_empty() => {
                variants[rng.gen_range(0..variants.len())].clone()
            }
            Some(_) => String::new(),
            None => {
                let len = rng.gen_range(1..=32);
                (0..len)
                    .map(|_| {
                        const CHARS: &[u8] =
                            b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
                        CHARS[rng.gen_range(0..CHARS.len())] as char
                    })
                    .collect()
            }
        };
        serde_json::Value::String(s)
    }

    pub fn generate_content_from_rng(
        &self,
        rng: &mut impl Rng,
        permutations: usize,
    ) -> (String, Vec<Logprob>) {
        let serialized: Vec<String> = (0..permutations)
            .map(|_| serde_json::to_string(&self.generate_value_from_rng(rng)).unwrap())
            .collect();
        generate_logprobs_from_serialized(&serialized, rng)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum NumberJsonSchemaType {
    Number,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NumberJsonSchema {
    r#type: NumberJsonSchemaType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
}

impl NumberJsonSchema {
    fn generate_value_from_rng(&self, rng: &mut impl Rng) -> serde_json::Value {
        let min = self.minimum.unwrap_or(0.0);
        let max = self.maximum.unwrap_or(100.0);
        serde_json::Value::Number(
            serde_json::Number::from_f64(rng.gen_range(min..=max)).unwrap_or_else(|| {
                serde_json::Number::from_f64(0.0).unwrap()
            }),
        )
    }

    pub fn generate_content_from_rng(
        &self,
        rng: &mut impl Rng,
        permutations: usize,
    ) -> (String, Vec<Logprob>) {
        let serialized: Vec<String> = (0..permutations)
            .map(|_| serde_json::to_string(&self.generate_value_from_rng(rng)).unwrap())
            .collect();
        generate_logprobs_from_serialized(&serialized, rng)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum IntegerJsonSchemaType {
    Integer,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IntegerJsonSchema {
    r#type: IntegerJsonSchemaType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<i64>,
}

impl IntegerJsonSchema {
    fn generate_value_from_rng(&self, rng: &mut impl Rng) -> serde_json::Value {
        let min = self.minimum.unwrap_or(0);
        let max = self.maximum.unwrap_or(100);
        serde_json::json!(rng.gen_range(min..=max))
    }

    pub fn generate_content_from_rng(
        &self,
        rng: &mut impl Rng,
        permutations: usize,
    ) -> (String, Vec<Logprob>) {
        let serialized: Vec<String> = (0..permutations)
            .map(|_| serde_json::to_string(&self.generate_value_from_rng(rng)).unwrap())
            .collect();
        generate_logprobs_from_serialized(&serialized, rng)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum BooleanJsonSchemaType {
    Boolean,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BooleanJsonSchema {
    r#type: BooleanJsonSchemaType,
}

impl BooleanJsonSchema {
    fn generate_value_from_rng(&self, rng: &mut impl Rng) -> serde_json::Value {
        serde_json::Value::Bool(rng.gen_bool(0.5))
    }

    pub fn generate_content_from_rng(
        &self,
        rng: &mut impl Rng,
        permutations: usize,
    ) -> (String, Vec<Logprob>) {
        let serialized: Vec<String> = (0..permutations)
            .map(|_| serde_json::to_string(&self.generate_value_from_rng(rng)).unwrap())
            .collect();
        generate_logprobs_from_serialized(&serialized, rng)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum ArrayJsonSchemaType {
    Array,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArrayJsonSchema {
    r#type: ArrayJsonSchemaType,
    pub items: Box<JsonSchema>,
    #[serde(rename = "minItems", skip_serializing_if = "Option::is_none")]
    pub min_items: Option<u64>,
    #[serde(rename = "maxItems", skip_serializing_if = "Option::is_none")]
    pub max_items: Option<u64>,
}

impl ArrayJsonSchema {
    fn length(&self, rng: &mut impl Rng) -> usize {
        let min = self.min_items.unwrap_or(1).max(1) as usize;
        let max = self.max_items.unwrap_or(10).max(min as u64) as usize;
        rng.gen_range(min..=max)
    }

    pub fn generate_content_from_rng(
        &self,
        rng: &mut impl Rng,
        permutations: usize,
    ) -> (String, Vec<Logprob>) {
        let len = self.length(rng);

        if len == 0 {
            return ("[]".to_string(), vec![structural_logprob("[]")]);
        }

        let mut content = String::from("[");
        let mut logprobs = vec![structural_logprob("[")];

        for i in 0..len {
            if i > 0 {
                content.push(',');
                logprobs.push(structural_logprob(","));
            }

            let (item_content, item_logprobs) =
                self.items.generate_content_from_rng(rng, permutations);
            content.push_str(&item_content);
            logprobs.extend(item_logprobs);
        }

        content.push(']');
        logprobs.push(structural_logprob("]"));

        (content, logprobs)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum ObjectJsonSchemaType {
    Object,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ObjectJsonSchema {
    r#type: ObjectJsonSchemaType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<IndexMap<String, JsonSchema>>,
}

impl ObjectJsonSchema {
    pub fn generate_content_from_rng(
        &self,
        rng: &mut impl Rng,
        permutations: usize,
    ) -> (String, Vec<Logprob>) {
        let props = match &self.properties {
            Some(props) if !props.is_empty() => props,
            _ => return ("{}".to_string(), vec![structural_logprob("{}")]),
        };

        let mut content = String::from("{");
        let mut logprobs = vec![structural_logprob("{")];

        for (i, (key, schema)) in props.iter().enumerate() {
            if i > 0 {
                content.push(',');
                logprobs.push(structural_logprob(","));
            }

            let key_token = format!("\"{}\":", serde_json::to_string(key).unwrap().trim_matches('"'));
            content.push_str(&key_token);
            logprobs.push(structural_logprob(&key_token));

            let (value_content, value_logprobs) = schema.generate_content_from_rng(rng, permutations);
            content.push_str(&value_content);
            logprobs.extend(value_logprobs);
        }

        content.push('}');
        logprobs.push(structural_logprob("}"));

        (content, logprobs)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnyOfJsonSchema {
    #[serde(rename = "anyOf")]
    pub any_of: Vec<JsonSchema>,
}

impl AnyOfJsonSchema {
    pub fn generate_content_from_rng(
        &self,
        rng: &mut impl Rng,
        permutations: usize,
    ) -> (String, Vec<Logprob>) {
        if self.any_of.is_empty() {
            return (String::new(), Vec::new());
        }
        let idx = rng.gen_range(0..self.any_of.len());
        self.any_of[idx].generate_content_from_rng(rng, permutations)
    }
}

/// Creates a logprob entry for structural JSON tokens (braces, commas, keys)
/// with probability 1.0 (logprob 0.0) and no alternatives.
fn structural_logprob(token: &str) -> Logprob {
    Logprob {
        token: token.to_string(),
        bytes: Some(token.as_bytes().to_vec()),
        logprob: Decimal::ZERO,
        top_logprobs: vec![TopLogprob {
            token: token.to_string(),
            bytes: Some(token.as_bytes().to_vec()),
            logprob: Some(Decimal::ZERO),
        }],
    }
}

/// Given a list of serialized JSON strings (the "permutations"), tokenize the
/// primary (first) string into 1-3 char tokens and build logprobs with
/// corresponding slices from all alternatives.
pub fn generate_logprobs_from_serialized(
    serialized: &[String],
    rng: &mut impl Rng,
) -> (String, Vec<Logprob>) {
    if serialized.is_empty() {
        return (String::new(), Vec::new());
    }

    let permutations = serialized.len();
    let primary = &serialized[0];
    let primary_bytes = primary.as_bytes();
    let mut logprobs = Vec::new();
    let mut pos = 0;

    while pos < primary_bytes.len() {
        // Token length: 1-3 chars, each equally likely
        let max_len = (primary_bytes.len() - pos).min(3);
        let token_len = if max_len == 1 {
            1
        } else {
            rng.gen_range(1..=max_len)
        };
        let token = std::str::from_utf8(&primary_bytes[pos..pos + token_len])
            .unwrap()
            .to_string();

        // Collect corresponding slices from all variants (including primary)
        let mut token_probs: Vec<(String, Decimal)> = Vec::with_capacity(permutations);

        // Generate raw random weights
        let mut raw_weights: Vec<f64> = (0..permutations)
            .map(|_| rng.gen_range(0.001f64..1.0))
            .collect();
        // Sort descending so first is highest
        raw_weights.sort_by(|a, b| b.partial_cmp(a).unwrap());
        let weight_sum: f64 = raw_weights.iter().sum();

        for (i, ser) in serialized.iter().enumerate() {
            let ser_bytes = ser.as_bytes();
            let alt_token = if pos < ser_bytes.len() {
                let end = (pos + token_len).min(ser_bytes.len());
                std::str::from_utf8(&ser_bytes[pos..end])
                    .unwrap()
                    .to_string()
            } else {
                // This variant is shorter; skip it and redistribute weight
                continue;
            };

            let prob = Decimal::try_from(raw_weights[i] / weight_sum).unwrap_or(Decimal::ZERO);
            token_probs.push((alt_token, prob));
        }

        // Redistribute weight from skipped variants to the primary
        let assigned: Decimal = token_probs.iter().map(|(_, p)| *p).sum();
        if assigned < Decimal::ONE && !token_probs.is_empty() {
            token_probs[0].1 += Decimal::ONE - assigned;
        }

        // Merge identical tokens
        let mut merged: Vec<(String, Decimal)> = Vec::with_capacity(token_probs.len());
        for (tok, prob) in token_probs {
            if let Some(existing) = merged.iter_mut().find(|(t, _)| *t == tok) {
                existing.1 += prob;
            } else {
                merged.push((tok, prob));
            }
        }

        // Sort by probability descending
        merged.sort_by(|a, b| b.1.cmp(&a.1));

        // The primary token's logprob is ln(probability)
        let primary_prob = merged[0].1;
        let logprob_value = if primary_prob > Decimal::ZERO {
            Decimal::try_from(
                f64::try_from(primary_prob).unwrap_or(1.0).ln()
            ).unwrap_or(Decimal::ZERO)
        } else {
            Decimal::new(-100, 0)
        };

        // Build top_logprobs from all entries (including primary)
        let top_logprobs: Vec<TopLogprob> = merged
            .iter()
            .map(|(tok, prob)| {
                let lp = if *prob > Decimal::ZERO {
                    Some(Decimal::try_from(
                        f64::try_from(*prob).unwrap_or(1.0).ln()
                    ).unwrap_or(Decimal::ZERO))
                } else {
                    Some(Decimal::new(-100, 0))
                };
                TopLogprob {
                    token: tok.clone(),
                    bytes: Some(tok.as_bytes().to_vec()),
                    logprob: lp,
                }
            })
            .collect();

        logprobs.push(Logprob {
            token: token.clone(),
            bytes: Some(token.as_bytes().to_vec()),
            logprob: logprob_value,
            top_logprobs,
        });

        pos += token_len;
    }

    (primary.clone(), logprobs)
}

#[cfg(test)]
#[path = "json_schema_tests.rs"]
mod tests;
