//! Input types for Function expressions.
//!
//! Defines the data structures that can be passed as input to Functions,
//! along with schema types for validation.

use crate::chat;
use indexmap::IndexMap;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Expressions that produce the 2D array used for mapped tasks.
///
/// Can be a single expression (producing one sub-array) or multiple
/// expressions (each producing a sub-array).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InputMaps {
    /// A single expression producing one sub-array.
    One(super::Expression),
    /// Multiple expressions, each producing a sub-array.
    Many(Vec<super::Expression>),
}

impl InputMaps {
    pub fn compile(
        self,
        params: &super::Params,
    ) -> Result<Vec<Vec<Input>>, super::ExpressionError> {
        match self {
            InputMaps::One(expression) => {
                match expression.compile_one_or_many::<Vec<Input>>(params)? {
                    super::OneOrMany::One(one) => Ok(vec![one]),
                    super::OneOrMany::Many(many) => Ok(many),
                }
            }
            InputMaps::Many(expressions) => {
                let mut compiled = Vec::with_capacity(expressions.len());
                for expression in expressions {
                    match expression
                        .compile_one_or_many::<Vec<Input>>(params)?
                    {
                        super::OneOrMany::One(one) => compiled.push(one),
                        super::OneOrMany::Many(many) => {
                            for item in many {
                                compiled.push(item);
                            }
                        }
                    }
                }
                Ok(compiled)
            }
        }
    }
}

/// A concrete input value (post-compilation).
///
/// Represents any JSON-like value that can be passed to a Function,
/// including rich content types (images, audio, video, files).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Input {
    /// Rich content (image, audio, video, file).
    RichContentPart(chat::completions::request::RichContentPart),
    /// An object with string keys.
    Object(IndexMap<String, Input>),
    /// An array of values.
    Array(Vec<Input>),
    /// A string value.
    String(String),
    /// An integer value.
    Integer(i64),
    /// A floating-point number.
    Number(f64),
    /// A boolean value.
    Boolean(bool),
}

/// An input value that may contain JMESPath expressions (pre-compilation).
///
/// Similar to [`Input`] but object values and array elements can be
/// expressions that are evaluated during compilation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InputExpression {
    /// Rich content (image, audio, video, file).
    RichContentPart(chat::completions::request::RichContentPart),
    /// An object with values that may be expressions.
    Object(IndexMap<String, super::WithExpression<InputExpression>>),
    /// An array with elements that may be expressions.
    Array(Vec<super::WithExpression<InputExpression>>),
    /// A string value.
    String(String),
    /// An integer value.
    Integer(i64),
    /// A floating-point number.
    Number(f64),
    /// A boolean value.
    Boolean(bool),
}

impl InputExpression {
    pub fn compile(
        self,
        params: &super::Params,
    ) -> Result<Input, super::ExpressionError> {
        match self {
            InputExpression::RichContentPart(rich_content_part) => {
                Ok(Input::RichContentPart(rich_content_part))
            }
            InputExpression::Object(object) => {
                let mut compiled_object = IndexMap::with_capacity(object.len());
                for (key, value) in object {
                    compiled_object.insert(
                        key,
                        value.compile_one(params)?.compile(params)?,
                    );
                }
                Ok(Input::Object(compiled_object))
            }
            InputExpression::Array(array) => {
                let mut compiled_array = Vec::with_capacity(array.len());
                for item in array {
                    match item.compile_one_or_many(params)? {
                        super::OneOrMany::One(one_item) => {
                            compiled_array.push(one_item.compile(params)?);
                        }
                        super::OneOrMany::Many(many_items) => {
                            for item in many_items {
                                compiled_array.push(item.compile(params)?);
                            }
                        }
                    }
                }
                Ok(Input::Array(compiled_array))
            }
            InputExpression::String(string) => Ok(Input::String(string)),
            InputExpression::Integer(integer) => Ok(Input::Integer(integer)),
            InputExpression::Number(number) => Ok(Input::Number(number)),
            InputExpression::Boolean(boolean) => Ok(Input::Boolean(boolean)),
        }
    }
}

/// Schema for validating Function input.
///
/// Defines the expected structure and constraints for input data.
/// Used by remote Functions to document and validate their inputs.
#[derive(Debug, Clone)]
pub enum InputSchema {
    /// An object with named properties.
    Object(ObjectInputSchema),
    /// An array of items.
    Array(ArrayInputSchema),
    /// A string value.
    String(StringInputSchema),
    /// An integer value.
    Integer(IntegerInputSchema),
    /// A floating-point number.
    Number(NumberInputSchema),
    /// A boolean value.
    Boolean(BooleanInputSchema),
    /// An image (URL or base64).
    Image(ImageInputSchema),
    /// Audio content.
    Audio(AudioInputSchema),
    /// Video content.
    Video(VideoInputSchema),
    /// A file.
    File(FileInputSchema),
    /// A union of schemas - input must match at least one.
    AnyOf(AnyOfInputSchema),
}

impl InputSchema {
    pub fn validate_input(&self, input: &Input) -> bool {
        match self {
            InputSchema::Object(schema) => schema.validate_input(input),
            InputSchema::Array(schema) => schema.validate_input(input),
            InputSchema::String(schema) => schema.validate_input(input),
            InputSchema::Integer(schema) => schema.validate_input(input),
            InputSchema::Number(schema) => schema.validate_input(input),
            InputSchema::Boolean(schema) => schema.validate_input(input),
            InputSchema::Image(schema) => schema.validate_input(input),
            InputSchema::Audio(schema) => schema.validate_input(input),
            InputSchema::Video(schema) => schema.validate_input(input),
            InputSchema::File(schema) => schema.validate_input(input),
            InputSchema::AnyOf(schema) => schema.validate_input(input),
        }
    }
}

/// Helper enum for deserializing typed schemas (those with a `type` field).
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum TypedInputSchema {
    Object(ObjectInputSchema),
    Array(ArrayInputSchema),
    String(StringInputSchema),
    Integer(IntegerInputSchema),
    Number(NumberInputSchema),
    Boolean(BooleanInputSchema),
    Image(ImageInputSchema),
    Audio(AudioInputSchema),
    Video(VideoInputSchema),
    File(FileInputSchema),
}

impl From<TypedInputSchema> for InputSchema {
    fn from(typed: TypedInputSchema) -> Self {
        match typed {
            TypedInputSchema::Object(s) => InputSchema::Object(s),
            TypedInputSchema::Array(s) => InputSchema::Array(s),
            TypedInputSchema::String(s) => InputSchema::String(s),
            TypedInputSchema::Integer(s) => InputSchema::Integer(s),
            TypedInputSchema::Number(s) => InputSchema::Number(s),
            TypedInputSchema::Boolean(s) => InputSchema::Boolean(s),
            TypedInputSchema::Image(s) => InputSchema::Image(s),
            TypedInputSchema::Audio(s) => InputSchema::Audio(s),
            TypedInputSchema::Video(s) => InputSchema::Video(s),
            TypedInputSchema::File(s) => InputSchema::File(s),
        }
    }
}

impl<'de> Deserialize<'de> for InputSchema {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;

        // Check if this is an AnyOf schema (has anyOf field, no type field)
        if value.get("anyOf").is_some() {
            let schema: AnyOfInputSchema =
                serde_json::from_value(value).map_err(D::Error::custom)?;
            Ok(InputSchema::AnyOf(schema))
        } else {
            // Deserialize as a typed schema
            let typed: TypedInputSchema =
                serde_json::from_value(value).map_err(D::Error::custom)?;
            Ok(typed.into())
        }
    }
}

impl Serialize for InputSchema {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            InputSchema::AnyOf(schema) => schema.serialize(serializer),
            InputSchema::Object(schema) => {
                #[derive(Serialize)]
                #[serde(rename_all = "camelCase")]
                struct Tagged<'a> {
                    r#type: &'static str,
                    #[serde(flatten)]
                    schema: &'a ObjectInputSchema,
                }
                Tagged {
                    r#type: "object",
                    schema,
                }
                .serialize(serializer)
            }
            InputSchema::Array(schema) => {
                #[derive(Serialize)]
                #[serde(rename_all = "camelCase")]
                struct Tagged<'a> {
                    r#type: &'static str,
                    #[serde(flatten)]
                    schema: &'a ArrayInputSchema,
                }
                Tagged {
                    r#type: "array",
                    schema,
                }
                .serialize(serializer)
            }
            InputSchema::String(schema) => {
                #[derive(Serialize)]
                #[serde(rename_all = "camelCase")]
                struct Tagged<'a> {
                    r#type: &'static str,
                    #[serde(flatten)]
                    schema: &'a StringInputSchema,
                }
                Tagged {
                    r#type: "string",
                    schema,
                }
                .serialize(serializer)
            }
            InputSchema::Integer(schema) => {
                #[derive(Serialize)]
                #[serde(rename_all = "camelCase")]
                struct Tagged<'a> {
                    r#type: &'static str,
                    #[serde(flatten)]
                    schema: &'a IntegerInputSchema,
                }
                Tagged {
                    r#type: "integer",
                    schema,
                }
                .serialize(serializer)
            }
            InputSchema::Number(schema) => {
                #[derive(Serialize)]
                #[serde(rename_all = "camelCase")]
                struct Tagged<'a> {
                    r#type: &'static str,
                    #[serde(flatten)]
                    schema: &'a NumberInputSchema,
                }
                Tagged {
                    r#type: "number",
                    schema,
                }
                .serialize(serializer)
            }
            InputSchema::Boolean(schema) => {
                #[derive(Serialize)]
                #[serde(rename_all = "camelCase")]
                struct Tagged<'a> {
                    r#type: &'static str,
                    #[serde(flatten)]
                    schema: &'a BooleanInputSchema,
                }
                Tagged {
                    r#type: "boolean",
                    schema,
                }
                .serialize(serializer)
            }
            InputSchema::Image(schema) => {
                #[derive(Serialize)]
                #[serde(rename_all = "camelCase")]
                struct Tagged<'a> {
                    r#type: &'static str,
                    #[serde(flatten)]
                    schema: &'a ImageInputSchema,
                }
                Tagged {
                    r#type: "image",
                    schema,
                }
                .serialize(serializer)
            }
            InputSchema::Audio(schema) => {
                #[derive(Serialize)]
                #[serde(rename_all = "camelCase")]
                struct Tagged<'a> {
                    r#type: &'static str,
                    #[serde(flatten)]
                    schema: &'a AudioInputSchema,
                }
                Tagged {
                    r#type: "audio",
                    schema,
                }
                .serialize(serializer)
            }
            InputSchema::Video(schema) => {
                #[derive(Serialize)]
                #[serde(rename_all = "camelCase")]
                struct Tagged<'a> {
                    r#type: &'static str,
                    #[serde(flatten)]
                    schema: &'a VideoInputSchema,
                }
                Tagged {
                    r#type: "video",
                    schema,
                }
                .serialize(serializer)
            }
            InputSchema::File(schema) => {
                #[derive(Serialize)]
                #[serde(rename_all = "camelCase")]
                struct Tagged<'a> {
                    r#type: &'static str,
                    #[serde(flatten)]
                    schema: &'a FileInputSchema,
                }
                Tagged {
                    r#type: "file",
                    schema,
                }
                .serialize(serializer)
            }
        }
    }
}

/// Schema for a union of possible types - input must match at least one.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnyOfInputSchema {
    /// The possible schemas that the input can match.
    pub any_of: Vec<InputSchema>,
}

impl AnyOfInputSchema {
    pub fn validate_input(&self, input: &Input) -> bool {
        self.any_of
            .iter()
            .any(|schema| schema.validate_input(input))
    }
}

/// Schema for an object input with named properties.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectInputSchema {
    /// Human-readable description of the object.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Schema for each property in the object.
    pub properties: IndexMap<String, InputSchema>,
    /// List of property names that must be present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
}

impl ObjectInputSchema {
    pub fn validate_input(&self, input: &Input) -> bool {
        match input {
            Input::Object(map) => {
                self.properties.iter().all(|(key, schema)| {
                    map.get(key)
                        .map(|value| schema.validate_input(value))
                        .unwrap_or(false)
                }) && {
                    if let Some(required) = &self.required {
                        required.iter().all(|key| map.contains_key(key))
                    } else {
                        true
                    }
                }
            }
            _ => false,
        }
    }
}

/// Schema for an array input.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArrayInputSchema {
    /// Human-readable description of the array.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Minimum number of items required.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_items: Option<u64>,
    /// Maximum number of items allowed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_items: Option<u64>,
    /// Schema for each item in the array.
    pub items: Box<InputSchema>,
}

impl ArrayInputSchema {
    pub fn validate_input(&self, input: &Input) -> bool {
        match input {
            Input::Array(array) => {
                if let Some(min_items) = self.min_items
                    && (array.len() as u64) < min_items
                {
                    false
                } else if let Some(max_items) = self.max_items
                    && (array.len() as u64) > max_items
                {
                    false
                } else {
                    array.iter().all(|item| self.items.validate_input(item))
                }
            }
            _ => false,
        }
    }
}

/// Schema for a string input.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StringInputSchema {
    /// Human-readable description of the string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// If provided, the string must be one of these values.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#enum: Option<Vec<String>>,
}

impl StringInputSchema {
    pub fn validate_input(&self, input: &Input) -> bool {
        match input {
            Input::String(s) => {
                if let Some(r#enum) = &self.r#enum {
                    r#enum.contains(s)
                } else {
                    true
                }
            }
            _ => false,
        }
    }
}

/// Schema for an integer input.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegerInputSchema {
    /// Human-readable description of the integer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Minimum allowed value (inclusive).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<i64>,
    /// Maximum allowed value (inclusive).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<i64>,
}

impl IntegerInputSchema {
    pub fn validate_input(&self, input: &Input) -> bool {
        match input {
            Input::Integer(integer) => {
                if let Some(minimum) = self.minimum
                    && *integer < minimum
                {
                    false
                } else if let Some(maximum) = self.maximum
                    && *integer > maximum
                {
                    false
                } else {
                    true
                }
            }
            Input::Number(number)
                if number.is_finite() && number.fract() == 0.0 =>
            {
                let integer = *number as i64;
                if let Some(minimum) = self.minimum
                    && integer < minimum
                {
                    false
                } else if let Some(maximum) = self.maximum
                    && integer > maximum
                {
                    false
                } else {
                    true
                }
            }
            _ => false,
        }
    }
}

/// Schema for a floating-point number input.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NumberInputSchema {
    /// Human-readable description of the number.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Minimum allowed value (inclusive).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    /// Maximum allowed value (inclusive).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
}

impl NumberInputSchema {
    pub fn validate_input(&self, input: &Input) -> bool {
        match input {
            Input::Integer(integer) => {
                let number = *integer as f64;
                if let Some(minimum) = self.minimum
                    && number < minimum
                {
                    false
                } else if let Some(maximum) = self.maximum
                    && number > maximum
                {
                    false
                } else {
                    true
                }
            }
            Input::Number(number) => {
                if let Some(minimum) = self.minimum
                    && *number < minimum
                {
                    false
                } else if let Some(maximum) = self.maximum
                    && *number > maximum
                {
                    false
                } else {
                    true
                }
            }
            _ => false,
        }
    }
}

/// Schema for a boolean input.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BooleanInputSchema {
    /// Human-readable description of the boolean.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl BooleanInputSchema {
    pub fn validate_input(&self, input: &Input) -> bool {
        match input {
            Input::Boolean(_) => true,
            _ => false,
        }
    }
}

/// Schema for an image input (URL or base64-encoded).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageInputSchema {
    /// Human-readable description of the expected image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl ImageInputSchema {
    pub fn validate_input(&self, input: &Input) -> bool {
        match input {
            Input::RichContentPart(
                chat::completions::request::RichContentPart::ImageUrl {
                    ..
                },
            ) => true,
            _ => false,
        }
    }
}

/// Schema for an audio input.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioInputSchema {
    /// Human-readable description of the expected audio.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl AudioInputSchema {
    pub fn validate_input(&self, input: &Input) -> bool {
        match input {
            Input::RichContentPart(
                chat::completions::request::RichContentPart::InputAudio {
                    ..
                },
            ) => true,
            _ => false,
        }
    }
}

/// Schema for a video input (URL or base64-encoded).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoInputSchema {
    /// Human-readable description of the expected video.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl VideoInputSchema {
    pub fn validate_input(&self, input: &Input) -> bool {
        match input {
            Input::RichContentPart(
                chat::completions::request::RichContentPart::InputVideo {
                    ..
                },
            ) => true,
            Input::RichContentPart(
                chat::completions::request::RichContentPart::VideoUrl {
                    ..
                },
            ) => true,
            _ => false,
        }
    }
}

/// Schema for a file input.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInputSchema {
    /// Human-readable description of the expected file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl FileInputSchema {
    pub fn validate_input(&self, input: &Input) -> bool {
        match input {
            Input::RichContentPart(
                chat::completions::request::RichContentPart::File { .. },
            ) => true,
            _ => false,
        }
    }
}
