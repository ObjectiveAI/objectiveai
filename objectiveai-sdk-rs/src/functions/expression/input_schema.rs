//! Schema types for validating Function input.
//!
//! Defines the expected structure and constraints for input data.
//! Used by remote Functions to document and validate their inputs.

use super::InputValue;
use crate::agent;
use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Schema for validating Function input.
///
/// Defines the expected structure and constraints for input data.
/// Used by remote Functions to document and validate their inputs.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(untagged)]
#[schemars(rename = "functions.expression.InputSchema")]
pub enum InputSchema {
    /// A union of schemas - input must match at least one.
    #[schemars(title = "AnyOf")]
    AnyOf(AnyOfInputSchema),
    /// An object with named properties.
    #[schemars(title = "Object")]
    Object(ObjectInputSchema),
    /// An array of items.
    #[schemars(title = "Array")]
    Array(ArrayInputSchema),
    /// A string value.
    #[schemars(title = "String")]
    String(StringInputSchema),
    /// An integer value.
    #[schemars(title = "Integer")]
    Integer(IntegerInputSchema),
    /// A floating-point number.
    #[schemars(title = "Number")]
    Number(NumberInputSchema),
    /// A boolean value.
    #[schemars(title = "Boolean")]
    Boolean(BooleanInputSchema),
    /// An image (URL or base64).
    #[schemars(title = "Image")]
    Image(ImageInputSchema),
    /// Audio content.
    #[schemars(title = "Audio")]
    Audio(AudioInputSchema),
    /// Video content.
    #[schemars(title = "Video")]
    Video(VideoInputSchema),
    /// A file.
    #[schemars(title = "File")]
    File(FileInputSchema),
}

impl InputSchema {
    /// Returns which media modalities are present anywhere in this schema.
    pub fn modalities(&self) -> Modalities {
        match self {
            InputSchema::Image(_) => Modalities {
                image: true,
                ..Modalities::default()
            },
            InputSchema::Audio(_) => Modalities {
                audio: true,
                ..Modalities::default()
            },
            InputSchema::Video(_) => Modalities {
                video: true,
                ..Modalities::default()
            },
            InputSchema::File(_) => Modalities {
                file: true,
                ..Modalities::default()
            },
            InputSchema::Object(s) => s.modalities(),
            InputSchema::Array(s) => s.modalities(),
            InputSchema::AnyOf(s) => s.modalities(),
            InputSchema::String(_)
            | InputSchema::Integer(_)
            | InputSchema::Number(_)
            | InputSchema::Boolean(_) => Modalities::default(),
        }
    }

    /// Validates that an input value conforms to this schema.
    pub fn validate_input(&self, input: &InputValue) -> bool {
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

/// Which media modalities are present in a schema.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Modalities {
    pub image: bool,
    pub audio: bool,
    pub video: bool,
    pub file: bool,
}

impl Modalities {
    /// Merge two `Modalities` (union).
    pub fn merge(self, other: Self) -> Self {
        Self {
            image: self.image || other.image,
            audio: self.audio || other.audio,
            video: self.video || other.video,
            file: self.file || other.file,
        }
    }
}

/// Schema for a union of possible types - input must match at least one.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(rename_all = "camelCase")]
#[schemars(rename = "functions.expression.AnyOfInputSchema")]
pub struct AnyOfInputSchema {
    /// The possible schemas that the input can match.
    pub any_of: Vec<InputSchema>,
}

impl AnyOfInputSchema {
    /// Returns which media modalities are present in any variant.
    pub fn modalities(&self) -> Modalities {
        self.any_of
            .iter()
            .fold(Modalities::default(), |acc, s| acc.merge(s.modalities()))
    }

    /// Validates that an input matches at least one schema in the union.
    pub fn validate_input(&self, input: &InputValue) -> bool {
        self.any_of
            .iter()
            .any(|schema| schema.validate_input(input))
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(rename_all = "lowercase")]
#[schemars(rename = "functions.expression.ObjectInputSchemaType")]
pub enum ObjectInputSchemaType {
    #[default]
    Object,
}

/// Schema for an object input with named properties.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(rename_all = "camelCase")]
#[schemars(rename = "functions.expression.ObjectInputSchema")]
pub struct ObjectInputSchema {
    pub r#type: ObjectInputSchemaType,
    /// Human-readable description of the object.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub description: Option<String>,
    /// Schema for each property in the object.
    #[arbitrary(with = crate::arbitrary_util::arbitrary_indexmap)]
    pub properties: IndexMap<String, InputSchema>,
    /// List of property names that must be present.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub required: Option<Vec<String>>,
}

impl ObjectInputSchema {
    /// Returns which media modalities are present in any property.
    pub fn modalities(&self) -> Modalities {
        self.properties
            .values()
            .fold(Modalities::default(), |acc, s| acc.merge(s.modalities()))
    }

    /// Validates that an input is an object matching this schema.
    pub fn validate_input(&self, input: &InputValue) -> bool {
        match input {
            InputValue::Object(map) => {
                let required = self.required.as_deref().unwrap_or(&[]);
                self.properties
                    .iter()
                    .all(|(key, schema)| match map.get(key) {
                        Some(value) => schema.validate_input(value),
                        None => !required.contains(key),
                    })
            }
            _ => false,
        }
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(rename_all = "lowercase")]
#[schemars(rename = "functions.expression.ArrayInputSchemaType")]
pub enum ArrayInputSchemaType {
    #[default]
    Array,
}

/// Schema for an array input.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(rename_all = "camelCase")]
#[schemars(rename = "functions.expression.ArrayInputSchema")]
pub struct ArrayInputSchema {
    pub r#type: ArrayInputSchemaType,
    /// Human-readable description of the array.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub description: Option<String>,
    /// Minimum number of items required.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_u64)]
    pub min_items: Option<u64>,
    /// Maximum number of items allowed.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_u64)]
    pub max_items: Option<u64>,
    /// Schema for each item in the array.
    pub items: Box<InputSchema>,
}

impl ArrayInputSchema {
    /// Returns which media modalities are present in the item schema.
    pub fn modalities(&self) -> Modalities {
        self.items.modalities()
    }

    /// Validates that an input is an array matching this schema.
    pub fn validate_input(&self, input: &InputValue) -> bool {
        match input {
            InputValue::Array(array) => {
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

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(rename_all = "lowercase")]
#[schemars(rename = "functions.expression.StringInputSchemaType")]
pub enum StringInputSchemaType {
    #[default]
    String,
}

/// Schema for a string input.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(rename_all = "camelCase")]
#[schemars(rename = "functions.expression.StringInputSchema")]
pub struct StringInputSchema {
    pub r#type: StringInputSchemaType,
    /// Human-readable description of the string.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub description: Option<String>,
    /// If provided, the string must be one of these values.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub r#enum: Option<Vec<String>>,
}

impl StringInputSchema {
    /// Validates that an input is a string matching this schema.
    pub fn validate_input(&self, input: &InputValue) -> bool {
        match input {
            InputValue::String(s) => {
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

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(rename_all = "lowercase")]
#[schemars(rename = "functions.expression.IntegerInputSchemaType")]
pub enum IntegerInputSchemaType {
    #[default]
    Integer,
}

/// Schema for an integer input.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(rename_all = "camelCase")]
#[schemars(rename = "functions.expression.IntegerInputSchema")]
pub struct IntegerInputSchema {
    pub r#type: IntegerInputSchemaType,
    /// Human-readable description of the integer.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub description: Option<String>,
    /// Minimum allowed value (inclusive).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_i64)]
    pub minimum: Option<i64>,
    /// Maximum allowed value (inclusive).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_i64)]
    pub maximum: Option<i64>,
}

impl IntegerInputSchema {
    /// Validates that an input is an integer matching this schema.
    pub fn validate_input(&self, input: &InputValue) -> bool {
        match input {
            InputValue::Integer(integer) => {
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
            InputValue::Number(number)
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

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(rename_all = "lowercase")]
#[schemars(rename = "functions.expression.NumberInputSchemaType")]
pub enum NumberInputSchemaType {
    #[default]
    Number,
}

/// Schema for a floating-point number input.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(rename_all = "camelCase")]
#[schemars(rename = "functions.expression.NumberInputSchema")]
pub struct NumberInputSchema {
    pub r#type: NumberInputSchemaType,
    /// Human-readable description of the number.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub description: Option<String>,
    /// Minimum allowed value (inclusive).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_f64)]
    pub minimum: Option<f64>,
    /// Maximum allowed value (inclusive).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_f64)]
    pub maximum: Option<f64>,
}

impl NumberInputSchema {
    /// Validates that an input is a number matching this schema.
    pub fn validate_input(&self, input: &InputValue) -> bool {
        match input {
            InputValue::Integer(integer) => {
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
            InputValue::Number(number) => {
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

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(rename_all = "lowercase")]
#[schemars(rename = "functions.expression.BooleanInputSchemaType")]
pub enum BooleanInputSchemaType {
    #[default]
    Boolean,
}

/// Schema for a boolean input.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(rename_all = "camelCase")]
#[schemars(rename = "functions.expression.BooleanInputSchema")]
pub struct BooleanInputSchema {
    pub r#type: BooleanInputSchemaType,
    /// Human-readable description of the boolean.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub description: Option<String>,
}

impl BooleanInputSchema {
    /// Validates that an input is a boolean.
    pub fn validate_input(&self, input: &InputValue) -> bool {
        match input {
            InputValue::Boolean(_) => true,
            _ => false,
        }
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(rename_all = "lowercase")]
#[schemars(rename = "functions.expression.ImageInputSchemaType")]
pub enum ImageInputSchemaType {
    #[default]
    Image,
}

/// Schema for an image input (URL or base64-encoded).
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(rename_all = "camelCase")]
#[schemars(rename = "functions.expression.ImageInputSchema")]
pub struct ImageInputSchema {
    pub r#type: ImageInputSchemaType,
    /// Human-readable description of the expected image.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub description: Option<String>,
}

impl ImageInputSchema {
    /// Validates that an input is an image.
    pub fn validate_input(&self, input: &InputValue) -> bool {
        match input {
            InputValue::RichContentPart(
                agent::completions::message::RichContentPart::ImageUrl {
                    ..
                },
            ) => true,
            _ => false,
        }
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(rename_all = "lowercase")]
#[schemars(rename = "functions.expression.AudioInputSchemaType")]
pub enum AudioInputSchemaType {
    #[default]
    Audio,
}

/// Schema for an audio input.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(rename_all = "camelCase")]
#[schemars(rename = "functions.expression.AudioInputSchema")]
pub struct AudioInputSchema {
    pub r#type: AudioInputSchemaType,
    /// Human-readable description of the expected audio.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub description: Option<String>,
}

impl AudioInputSchema {
    /// Validates that an input is audio content.
    pub fn validate_input(&self, input: &InputValue) -> bool {
        match input {
            InputValue::RichContentPart(
                agent::completions::message::RichContentPart::InputAudio {
                    ..
                },
            ) => true,
            _ => false,
        }
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(rename_all = "lowercase")]
#[schemars(rename = "functions.expression.VideoInputSchemaType")]
pub enum VideoInputSchemaType {
    #[default]
    Video,
}

/// Schema for a video input (URL or base64-encoded).
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(rename_all = "camelCase")]
#[schemars(rename = "functions.expression.VideoInputSchema")]
pub struct VideoInputSchema {
    pub r#type: VideoInputSchemaType,
    /// Human-readable description of the expected video.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub description: Option<String>,
}

impl VideoInputSchema {
    /// Validates that an input is video content.
    pub fn validate_input(&self, input: &InputValue) -> bool {
        match input {
            InputValue::RichContentPart(
                agent::completions::message::RichContentPart::InputVideo {
                    ..
                },
            ) => true,
            InputValue::RichContentPart(
                agent::completions::message::RichContentPart::VideoUrl {
                    ..
                },
            ) => true,
            _ => false,
        }
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(rename_all = "lowercase")]
#[schemars(rename = "functions.expression.FileInputSchemaType")]
pub enum FileInputSchemaType {
    #[default]
    File,
}

/// Schema for a file input.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(rename_all = "camelCase")]
#[schemars(rename = "functions.expression.FileInputSchema")]
pub struct FileInputSchema {
    pub r#type: FileInputSchemaType,
    /// Human-readable description of the expected file.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub description: Option<String>,
}

impl FileInputSchema {
    /// Validates that an input is a file.
    pub fn validate_input(&self, input: &InputValue) -> bool {
        match input {
            InputValue::RichContentPart(
                agent::completions::message::RichContentPart::File { .. },
            ) => true,
            _ => false,
        }
    }
}
