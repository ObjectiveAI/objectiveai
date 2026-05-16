//! Input types for Function expressions.
//!
//! Defines the data structures that can be passed as input to Functions,
//! along with schema types for validation.

use crate::agent;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use starlark::values::dict::DictRef as StarlarkDictRef;
use starlark::values::float::UnpackFloat;
use starlark::values::list::ListRef as StarlarkListRef;
use starlark::values::{Heap as StarlarkHeap, UnpackValue, Value as StarlarkValue};
use schemars::JsonSchema;

/// A concrete input value (post-compilation).
///
/// Represents any JSON-like value that can be passed to a Function,
/// including rich content types (images, audio, video, files).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "functions.expression.InputValue")]
pub enum InputValue {
    /// Rich content (image, audio, video, file).
    #[schemars(title = "RichContentPart")]
    RichContentPart(agent::completions::message::RichContentPart),
    /// An object with string keys.
    #[schemars(title = "Object")]
    Object(IndexMap<String, InputValue>),
    /// An array of values.
    #[schemars(title = "Array")]
    Array(Vec<InputValue>),
    /// A string value.
    #[schemars(title = "String")]
    String(String),
    /// An integer value.
    #[schemars(title = "Integer")]
    Integer(i64),
    /// A floating-point number.
    #[schemars(title = "Number")]
    Number(f64),
    /// A boolean value.
    #[schemars(title = "Boolean")]
    Boolean(bool),
}

impl super::ToStarlarkValue for InputValue {
    fn to_starlark_value<'v>(&self, heap: &'v StarlarkHeap) -> StarlarkValue<'v> {
        match self {
            InputValue::String(s) => s.to_starlark_value(heap),
            InputValue::Integer(i) => i.to_starlark_value(heap),
            InputValue::Number(f) => f.to_starlark_value(heap),
            InputValue::Boolean(b) => b.to_starlark_value(heap),
            InputValue::Object(map) => map.to_starlark_value(heap),
            InputValue::Array(arr) => arr.to_starlark_value(heap),
            InputValue::RichContentPart(part) => part.to_starlark_value(heap),
        }
    }
}

impl super::FromStarlarkValue for InputValue {
    fn from_starlark_value(value: &StarlarkValue) -> Result<Self, super::ExpressionError> {
        if value.is_none() {
            return Err(super::ExpressionError::StarlarkConversionError("Input: expected value".into()));
        }
        if let Ok(Some(b)) = bool::unpack_value(*value) {
            return Ok(InputValue::Boolean(b));
        }
        if let Ok(Some(i)) = i64::unpack_value(*value) {
            return Ok(InputValue::Integer(i));
        }
        if let Ok(Some(UnpackFloat(f))) = UnpackFloat::unpack_value(*value) {
            return Ok(InputValue::Number(f));
        }
        if let Ok(Some(s)) = <&str as UnpackValue>::unpack_value(*value) {
            return Ok(InputValue::String(s.to_owned()));
        }
        if let Some(list) = StarlarkListRef::from_value(*value) {
            let mut items = Vec::with_capacity(list.len());
            for v in list.iter() {
                items.push(InputValue::from_starlark_value(&v)?);
            }
            return Ok(InputValue::Array(items));
        }
        if let Some(dict) = StarlarkDictRef::from_value(*value) {
            // Try RichContentPart first if "type" matches a known variant
            let mut type_value = None;
            for (k, v) in dict.iter() {
                if let Ok(Some("type")) = <&str as UnpackValue>::unpack_value(k) {
                    type_value = <&str as UnpackValue>::unpack_value(v).ok().flatten();
                    break;
                }
            }
            if matches!(type_value, Some("text" | "image_url" | "input_audio" | "input_video" | "video_url" | "file")) {
                if let Ok(part) = agent::completions::message::RichContentPart::from_starlark_value(value) {
                    return Ok(InputValue::RichContentPart(part));
                }
            }
            let mut map = IndexMap::with_capacity(dict.len());
            for (k, v) in dict.iter() {
                let key = <&str as UnpackValue>::unpack_value(k)
                    .map_err(|e| super::ExpressionError::StarlarkConversionError(e.to_string()))?
                    .ok_or_else(|| super::ExpressionError::StarlarkConversionError("Input: expected string key".into()))?
                    .to_owned();
                map.insert(key, InputValue::from_starlark_value(&v)?);
            }
            return Ok(InputValue::Object(map));
        }
        Err(super::ExpressionError::StarlarkConversionError(format!(
            "Input: unsupported type: {}",
            value.get_type()
        )))
    }
}

impl super::FromSpecial for InputValue {
    fn from_special(
        special: &super::Special,
        params: &super::Params,
    ) -> Result<Self, super::ExpressionError> {
        match special {
            super::Special::Input => {
                let input = match params {
                    super::Params::Owned(o) => &o.input,
                    super::Params::Ref(r) => r.input,
                };
                Ok(input.clone())
            }
            super::Special::InputItemsOptionalContextMerge => {
                // Expects input to be an array of objects, each with 'items' (array)
                // and optionally 'context'. Merges all items into one array and takes
                // context from the first element.
                let input = match params {
                    super::Params::Owned(o) => &o.input,
                    super::Params::Ref(r) => r.input,
                };
                let InputValue::Array(arr) = input else {
                    return Err(super::ExpressionError::UnsupportedSpecial);
                };
                let mut merged_items = Vec::new();
                let mut context = None;
                for (i, elem) in arr.iter().enumerate() {
                    let InputValue::Object(obj) = elem else {
                        return Err(super::ExpressionError::UnsupportedSpecial);
                    };
                    if let Some(InputValue::Array(items)) = obj.get("items") {
                        merged_items.extend(items.iter().cloned());
                    }
                    if i == 0 {
                        context = obj.get("context").cloned();
                    }
                }
                let mut result = IndexMap::new();
                result.insert("items".to_string(), InputValue::Array(merged_items));
                if let Some(ctx) = context {
                    result.insert("context".to_string(), ctx);
                }
                Ok(InputValue::Object(result))
            }
            _ => Err(super::ExpressionError::UnsupportedSpecial),
        }
    }
}

impl super::FromSpecial for Vec<InputValue> {
    fn from_special(
        special: &super::Special,
        params: &super::Params,
    ) -> Result<Self, super::ExpressionError> {
        match special {
            super::Special::InputItemsOptionalContextSplit => {
                // Expects input to be an object with 'items' (array) and optionally
                // 'context'. Returns an array of objects, each with a 1-element 'items'
                // array and a duplicated 'context' if present.
                let input = match params {
                    super::Params::Owned(o) => &o.input,
                    super::Params::Ref(r) => r.input,
                };
                let InputValue::Object(obj) = input else {
                    return Err(super::ExpressionError::UnsupportedSpecial);
                };
                let Some(InputValue::Array(items)) = obj.get("items") else {
                    return Err(super::ExpressionError::UnsupportedSpecial);
                };
                let context = obj.get("context");
                let mut result = Vec::with_capacity(items.len());
                for item in items {
                    let mut sub = IndexMap::new();
                    sub.insert("items".to_string(), InputValue::Array(vec![item.clone()]));
                    if let Some(ctx) = context {
                        sub.insert("context".to_string(), ctx.clone());
                    }
                    result.push(InputValue::Object(sub));
                }
                Ok(result)
            }
            _ => Err(super::ExpressionError::UnsupportedSpecial),
        }
    }
}

impl Eq for InputValue {}

impl std::hash::Hash for InputValue {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            InputValue::RichContentPart(p) => p.hash(state),
            InputValue::Object(map) => {
                map.len().hash(state);
                for (k, v) in map {
                    k.hash(state);
                    v.hash(state);
                }
            }
            InputValue::Array(arr) => arr.hash(state),
            InputValue::String(s) => s.hash(state),
            InputValue::Integer(i) => i.hash(state),
            InputValue::Number(f) => canonical_f64_bits(*f).hash(state),
            InputValue::Boolean(b) => b.hash(state),
        }
    }
}

impl<'a> arbitrary::Arbitrary<'a> for InputValue {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        if u.arbitrary().unwrap_or(false) {
            // Compound value
            if u.arbitrary()? {
                let mut map = IndexMap::new();
                while u.arbitrary().unwrap_or(false) {
                    map.insert(u.arbitrary::<String>()?, u.arbitrary()?);
                }
                Ok(InputValue::Object(map))
            } else {
                let mut arr = Vec::new();
                while u.arbitrary().unwrap_or(false) {
                    arr.push(InputValue::arbitrary(u)?);
                }
                Ok(InputValue::Array(arr))
            }
        } else {
            // Leaf value
            match u.int_in_range(0..=4)? {
                0 => Ok(InputValue::RichContentPart(u.arbitrary()?)),
                1 => Ok(InputValue::String(u.arbitrary()?)),
                2 => Ok(InputValue::Integer(crate::arbitrary_util::arbitrary_i64(u)?)),
                3 => Ok(InputValue::Number(crate::arbitrary_util::arbitrary_f64(u)?)),
                _ => Ok(InputValue::Boolean(u.arbitrary()?)),
            }
        }
    }
}

/// Normalizes f64 to canonical bits for consistent hashing.
///
/// - NaN (any bit pattern) → single canonical value
/// - -0.0 → +0.0
/// - Everything else (including ±inf) → to_bits()
fn canonical_f64_bits(f: f64) -> u64 {
    if f.is_nan() {
        // All NaN patterns hash the same
        0x7FF8_0000_0000_0000 // canonical quiet NaN
    } else if f == 0.0 {
        // +0.0 and -0.0 hash the same
        0u64
    } else {
        f.to_bits()
    }
}

impl InputValue {
    /// Converts the input to a sequence of rich content parts.
    ///
    /// This is used to render structured input data as formatted JSON
    /// in multimodal messages.
    pub fn to_rich_content_parts(
        self,
        depth: usize,
    ) -> impl Iterator<Item = agent::completions::message::RichContentPart> {
        enum Iter {
            RichContentPart(RichContentPartIter),
            Object(Box<ObjectIter>),
            Array(Box<ArrayIter>),
            Primitive(Option<String>),
        }
        impl Iter {
            pub fn new(input: InputValue, depth: usize) -> Self {
                match input {
                    InputValue::RichContentPart(rich_content_part) => {
                        Iter::RichContentPart(RichContentPartIter {
                            first: true,
                            part: Some(rich_content_part),
                            last: true,
                        })
                    }
                    InputValue::Object(object) => {
                        Iter::Object(Box::new(ObjectIter {
                            object: object.into_iter(),
                            first: true,
                            child: None,
                            depth,
                        }))
                    }
                    InputValue::Array(array) => Iter::Array(Box::new(ArrayIter {
                        array: array.into_iter(),
                        first: true,
                        child: None,
                        depth,
                    })),
                    InputValue::String(string) => Iter::Primitive(Some(format!(
                        "\"{}\"",
                        json_escape::escape_str(&string),
                    ))),
                    InputValue::Integer(integer) => {
                        Iter::Primitive(Some(integer.to_string()))
                    }
                    InputValue::Number(number) => {
                        Iter::Primitive(Some(number.to_string()))
                    }
                    InputValue::Boolean(boolean) => {
                        Iter::Primitive(Some(boolean.to_string()))
                    }
                }
            }
        }
        impl Iterator for Iter {
            type Item = agent::completions::message::RichContentPart;
            fn next(&mut self) -> Option<Self::Item> {
                match self {
                    Iter::RichContentPart(rich_content_part_iter) => {
                        rich_content_part_iter.next()
                    }
                    Iter::Object(object_iter) => object_iter.next(),
                    Iter::Array(array_iter) => array_iter.next(),
                    Iter::Primitive(primitive_option) => {
                        primitive_option.take().map(|text| {
                            agent::completions::message::RichContentPart::Text {
                                text,
                            }
                        })
                    }
                }
            }
        }
        struct RichContentPartIter {
            first: bool,
            part: Option<agent::completions::message::RichContentPart>,
            last: bool,
        }
        impl Iterator for RichContentPartIter {
            type Item = agent::completions::message::RichContentPart;
            fn next(&mut self) -> Option<Self::Item> {
                if self.first {
                    self.first = false;
                    Some(agent::completions::message::RichContentPart::Text {
                        text: '"'.to_string(),
                    })
                } else if let Some(part) = self.part.take() {
                    Some(part)
                } else if self.last {
                    self.last = false;
                    Some(agent::completions::message::RichContentPart::Text {
                        text: '"'.to_string(),
                    })
                } else {
                    None
                }
            }
        }
        struct ObjectIter {
            object: indexmap::map::IntoIter<String, InputValue>,
            first: bool,
            child: Option<Iter>,
            depth: usize,
        }
        impl Iterator for ObjectIter {
            type Item = agent::completions::message::RichContentPart;
            fn next(&mut self) -> Option<Self::Item> {
                if self.first {
                    self.first = false;
                    if let Some((key, input)) = self.object.next() {
                        self.child = Some(Iter::new(input, self.depth + 1));
                        Some(
                            agent::completions::message::RichContentPart::Text {
                                text: format!(
                                    "{{\n{}\"{}\": ",
                                    "    ".repeat(self.depth + 1),
                                    key,
                                ),
                            },
                        )
                    } else {
                        Some(
                            agent::completions::message::RichContentPart::Text {
                                text: format!("{{}}"),
                            },
                        )
                    }
                } else if let Some(child) = &mut self.child {
                    if let Some(part) = child.next() {
                        Some(part)
                    } else if let Some((key, input)) = self.object.next() {
                        self.child = Some(Iter::new(input, self.depth + 1));
                        Some(
                            agent::completions::message::RichContentPart::Text {
                                text: format!(
                                    ",\n{}\"{}\": ",
                                    "    ".repeat(self.depth + 1),
                                    key,
                                ),
                            },
                        )
                    } else {
                        self.child = None;
                        Some(
                            agent::completions::message::RichContentPart::Text {
                                text: format!(
                                    "\n{}}}",
                                    "    ".repeat(self.depth)
                                ),
                            },
                        )
                    }
                } else {
                    None
                }
            }
        }
        struct ArrayIter {
            array: std::vec::IntoIter<InputValue>,
            first: bool,
            child: Option<Iter>,
            depth: usize,
        }
        impl Iterator for ArrayIter {
            type Item = agent::completions::message::RichContentPart;
            fn next(&mut self) -> Option<Self::Item> {
                if self.first {
                    self.first = false;
                    if let Some(input) = self.array.next() {
                        self.child = Some(Iter::new(input, self.depth + 1));
                        Some(
                            agent::completions::message::RichContentPart::Text {
                                text: format!(
                                    "[\n{}",
                                    "    ".repeat(self.depth + 1)
                                ),
                            },
                        )
                    } else {
                        Some(
                            agent::completions::message::RichContentPart::Text {
                                text: format!("[]"),
                            },
                        )
                    }
                } else if let Some(child) = &mut self.child {
                    if let Some(part) = child.next() {
                        Some(part)
                    } else if let Some(input) = self.array.next() {
                        self.child = Some(Iter::new(input, self.depth + 1));
                        Some(
                            agent::completions::message::RichContentPart::Text {
                                text: format!(
                                    ",\n{}",
                                    "    ".repeat(self.depth + 1),
                                ),
                            },
                        )
                    } else {
                        self.child = None;
                        Some(
                            agent::completions::message::RichContentPart::Text {
                                text: format!(
                                    "\n{}]",
                                    "    ".repeat(self.depth)
                                ),
                            },
                        )
                    }
                } else {
                    None
                }
            }
        }
        Iter::new(self, depth)
    }
}

/// An input value that may contain expressions (pre-compilation).
///
/// Similar to [`InputValue`] but object values and array elements can be
/// expressions (JMESPath or Starlark) that are evaluated during compilation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "functions.expression.InputValueExpression")]
pub enum InputValueExpression {
    /// Rich content (image, audio, video, file).
    #[schemars(title = "RichContentPart")]
    RichContentPart(agent::completions::message::RichContentPart),
    /// An object with values that may be expressions.
    #[schemars(title = "Object")]
    Object(IndexMap<String, super::WithExpression<InputValueExpression>>),
    /// An array with elements that may be expressions.
    #[schemars(title = "Array")]
    Array(Vec<super::WithExpression<InputValueExpression>>),
    /// A string value.
    #[schemars(title = "String")]
    String(String),
    /// An integer value.
    #[schemars(title = "Integer")]
    Integer(i64),
    /// A floating-point number.
    #[schemars(title = "Number")]
    Number(f64),
    /// A boolean value.
    #[schemars(title = "Boolean")]
    Boolean(bool),
}

impl InputValueExpression {
    /// Compiles the expression into a concrete [`InputValue`].
    pub fn compile(
        self,
        params: &super::Params,
    ) -> Result<InputValue, super::ExpressionError> {
        match self {
            InputValueExpression::RichContentPart(rich_content_part) => {
                Ok(InputValue::RichContentPart(rich_content_part))
            }
            InputValueExpression::Object(object) => {
                let mut compiled_object = IndexMap::with_capacity(object.len());
                for (key, value) in object {
                    compiled_object.insert(
                        key,
                        value.compile_one(params)?.compile(params)?,
                    );
                }
                Ok(InputValue::Object(compiled_object))
            }
            InputValueExpression::Array(array) => {
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
                Ok(InputValue::Array(compiled_array))
            }
            InputValueExpression::String(string) => Ok(InputValue::String(string)),
            InputValueExpression::Integer(integer) => Ok(InputValue::Integer(integer)),
            InputValueExpression::Number(number) => Ok(InputValue::Number(number)),
            InputValueExpression::Boolean(boolean) => Ok(InputValue::Boolean(boolean)),
        }
    }
}

impl super::FromStarlarkValue for InputValueExpression {
    fn from_starlark_value(value: &StarlarkValue) -> Result<Self, super::ExpressionError> {
        if value.is_none() {
            return Err(super::ExpressionError::StarlarkConversionError("InputValueExpression: expected value".into()));
        }
        if let Ok(Some(b)) = bool::unpack_value(*value) {
            return Ok(InputValueExpression::Boolean(b));
        }
        if let Ok(Some(i)) = i64::unpack_value(*value) {
            return Ok(InputValueExpression::Integer(i));
        }
        if let Ok(Some(UnpackFloat(f))) = UnpackFloat::unpack_value(*value) {
            return Ok(InputValueExpression::Number(f));
        }
        if let Ok(Some(s)) = <&str as UnpackValue>::unpack_value(*value) {
            return Ok(InputValueExpression::String(s.to_owned()));
        }
        if let Some(list) = StarlarkListRef::from_value(*value) {
            let mut items = Vec::with_capacity(list.len());
            for v in list.iter() {
                items.push(super::WithExpression::Value(InputValueExpression::from_starlark_value(&v)?));
            }
            return Ok(InputValueExpression::Array(items));
        }
        if let Some(dict) = StarlarkDictRef::from_value(*value) {
            // Try RichContentPart first if "type" matches a known variant
            let mut type_value = None;
            for (k, v) in dict.iter() {
                if let Ok(Some("type")) = <&str as UnpackValue>::unpack_value(k) {
                    type_value = <&str as UnpackValue>::unpack_value(v).ok().flatten();
                    break;
                }
            }
            if matches!(type_value, Some("text" | "image_url" | "input_audio" | "input_video" | "video_url" | "file")) {
                if let Ok(part) = agent::completions::message::RichContentPart::from_starlark_value(value) {
                    return Ok(InputValueExpression::RichContentPart(part));
                }
            }
            let mut map = IndexMap::with_capacity(dict.len());
            for (k, v) in dict.iter() {
                let key = <&str as UnpackValue>::unpack_value(k)
                    .map_err(|e| super::ExpressionError::StarlarkConversionError(e.to_string()))?
                    .ok_or_else(|| super::ExpressionError::StarlarkConversionError("InputValueExpression: expected string key".into()))?
                    .to_owned();
                map.insert(key, super::WithExpression::Value(InputValueExpression::from_starlark_value(&v)?));
            }
            return Ok(InputValueExpression::Object(map));
        }
        Err(super::ExpressionError::StarlarkConversionError(format!(
            "InputValueExpression: unsupported type: {}",
            value.get_type()
        )))
    }
}

impl<'a> arbitrary::Arbitrary<'a> for InputValueExpression {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        if u.arbitrary().unwrap_or(false) {
            // Compound value
            if u.arbitrary()? {
                let mut map = IndexMap::new();
                while u.arbitrary().unwrap_or(false) {
                    map.insert(u.arbitrary::<String>()?, u.arbitrary()?);
                }
                Ok(InputValueExpression::Object(map))
            } else {
                let mut arr = Vec::new();
                while u.arbitrary().unwrap_or(false) {
                    arr.push(u.arbitrary()?);
                }
                Ok(InputValueExpression::Array(arr))
            }
        } else {
            // Leaf value
            match u.int_in_range(0..=4)? {
                0 => Ok(InputValueExpression::RichContentPart(u.arbitrary()?)),
                1 => Ok(InputValueExpression::String(u.arbitrary()?)),
                2 => Ok(InputValueExpression::Integer(crate::arbitrary_util::arbitrary_i64(u)?)),
                3 => Ok(InputValueExpression::Number(crate::arbitrary_util::arbitrary_f64(u)?)),
                _ => Ok(InputValueExpression::Boolean(u.arbitrary()?)),
            }
        }
    }
}

impl super::FromSpecial for InputValueExpression {
    fn from_special(
        special: &super::Special,
        params: &super::Params,
    ) -> Result<Self, super::ExpressionError> {
        let input = InputValue::from_special(special, params)?;
        Ok(input_to_input_expression(input))
    }
}

fn input_to_input_expression(input: InputValue) -> InputValueExpression {
    match input {
        InputValue::RichContentPart(p) => InputValueExpression::RichContentPart(p),
        InputValue::Object(map) => InputValueExpression::Object(
            map.into_iter()
                .map(|(k, v)| {
                    (
                        k,
                        super::WithExpression::Value(input_to_input_expression(v)),
                    )
                })
                .collect(),
        ),
        InputValue::Array(arr) => InputValueExpression::Array(
            arr.into_iter()
                .map(|v| super::WithExpression::Value(input_to_input_expression(v)))
                .collect(),
        ),
        InputValue::String(s) => InputValueExpression::String(s),
        InputValue::Integer(i) => InputValueExpression::Integer(i),
        InputValue::Number(n) => InputValueExpression::Number(n),
        InputValue::Boolean(b) => InputValueExpression::Boolean(b),
    }
}
