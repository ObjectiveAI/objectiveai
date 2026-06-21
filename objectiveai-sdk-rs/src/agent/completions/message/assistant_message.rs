//! Assistant message and tool call types.

use super::rich_content::{RichContent, RichContentExpression};
use crate::functions;
use functions::expression::{
    ExpressionError, FromStarlarkValue, WithExpression,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use starlark::values::dict::DictRef as StarlarkDictRef;
use starlark::values::{UnpackValue, Value as StarlarkValue};

/// An assistant message (model's previous response).
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.completions.message.AssistantMessage")]
pub struct AssistantMessage {
    /// The message content, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub content: Option<RichContent>,
    /// Refusal message if the model declined to respond.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub refusal: Option<String>,
    /// Tool calls made by the assistant.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub tool_calls: Option<Vec<AssistantToolCall>>,
    /// Reasoning content from models that support chain-of-thought.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub reasoning: Option<String>,
}

impl AssistantMessage {
    /// Prepares the message by normalizing content and optional fields.
    pub fn prepare(&mut self) {
        if let Some(content) = &mut self.content {
            content.prepare();
            if content.is_empty() {
                self.content = None;
            }
        }
        if self.refusal.as_ref().is_some_and(String::is_empty) {
            self.refusal = None;
        }
        if let Some(tool_calls) = &mut self.tool_calls {
            tool_calls.retain(|tool_call| !tool_call.is_empty());
            if tool_calls.is_empty() {
                self.tool_calls = None;
            }
        }
        if self.reasoning.as_ref().is_some_and(String::is_empty) {
            self.reasoning = None;
        }
    }

}

impl FromStarlarkValue for AssistantMessage {
    fn from_starlark_value(
        value: &StarlarkValue,
    ) -> Result<Self, ExpressionError> {
        let dict = StarlarkDictRef::from_value(*value).ok_or_else(|| {
            ExpressionError::StarlarkConversionError(
                "AssistantMessage: expected dict".into(),
            )
        })?;
        let mut content = None;
        let mut refusal = None;
        let mut tool_calls = None;
        let mut reasoning = None;
        for (k, v) in dict.iter() {
            let key = <&str as UnpackValue>::unpack_value(k)
                .map_err(|e| {
                    ExpressionError::StarlarkConversionError(e.to_string())
                })?
                .ok_or_else(|| {
                    ExpressionError::StarlarkConversionError(
                        "AssistantMessage: expected string key".into(),
                    )
                })?;
            match key {
                "content" => {
                    content = Option::<RichContent>::from_starlark_value(&v)?
                }
                "refusal" => {
                    refusal = Option::<String>::from_starlark_value(&v)?
                }
                "tool_calls" => {
                    tool_calls =
                        Option::<Vec<AssistantToolCall>>::from_starlark_value(
                            &v,
                        )?
                }
                "reasoning" => {
                    reasoning = Option::<String>::from_starlark_value(&v)?
                }
                _ => {}
            }
        }
        Ok(AssistantMessage {
            content,
            refusal,
            tool_calls,
            reasoning,
        })
    }
}

/// Expression variant of [`AssistantMessage`] for dynamic content.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.completions.message.AssistantMessageExpression")]
pub struct AssistantMessageExpression {
    /// The content expression.
    #[serde(
        default,
        skip_serializing_if = "functions::expression::WithExpression::is_none"
    )]
    #[schemars(with = "Option<functions::expression::WithExpression<RichContentExpression>>", extend("omitempty" = true))]
    pub content:
        functions::expression::WithExpression<Option<RichContentExpression>>,
    #[serde(
        default,
        skip_serializing_if = "functions::expression::WithExpression::is_none"
    )]
    #[schemars(with = "Option<functions::expression::WithExpression<String>>", extend("omitempty" = true))]
    pub refusal: functions::expression::WithExpression<Option<String>>,
    #[serde(
        default,
        skip_serializing_if = "functions::expression::WithExpression::is_none"
    )]
    #[schemars(with = "Option<functions::expression::WithExpression<Vec<functions::expression::WithExpression<AssistantToolCallExpression>>>>", extend("omitempty" = true))]
    pub tool_calls: functions::expression::WithExpression<
        Option<
            Vec<
                functions::expression::WithExpression<
                    AssistantToolCallExpression,
                >,
            >,
        >,
    >,
    #[serde(
        default,
        skip_serializing_if = "functions::expression::WithExpression::is_none"
    )]
    #[schemars(with = "Option<functions::expression::WithExpression<String>>", extend("omitempty" = true))]
    pub reasoning: functions::expression::WithExpression<Option<String>>,
}

impl AssistantMessageExpression {
    /// Compiles the expression into a concrete [`AssistantMessage`].
    pub fn compile(
        self,
        params: &functions::expression::Params,
    ) -> Result<AssistantMessage, functions::expression::ExpressionError> {
        let content = self
            .content
            .compile_one(params)?
            .map(|content| content.compile(params))
            .transpose()?;
        let refusal = self.refusal.compile_one(params)?;
        let tool_calls = self
            .tool_calls
            .compile_one(params)?
            .map(|tool_calls| {
                let mut compiled_tool_calls =
                    Vec::with_capacity(tool_calls.len());
                for tool_call in tool_calls {
                    match tool_call.compile_one_or_many(params)? {
                        functions::expression::OneOrMany::One(
                            one_tool_call,
                        ) => {
                            compiled_tool_calls
                                .push(one_tool_call.compile(params)?);
                        }
                        functions::expression::OneOrMany::Many(
                            many_tool_calls,
                        ) => {
                            for tool_call in many_tool_calls {
                                compiled_tool_calls
                                    .push(tool_call.compile(params)?);
                            }
                        }
                    }
                }
                Ok::<_, functions::expression::ExpressionError>(
                    compiled_tool_calls,
                )
            })
            .transpose()?;
        let reasoning = self.reasoning.compile_one(params)?;
        Ok(AssistantMessage {
            content,
            refusal,
            tool_calls,
            reasoning,
        })
    }
}

impl FromStarlarkValue for AssistantMessageExpression {
    fn from_starlark_value(
        value: &StarlarkValue,
    ) -> Result<Self, ExpressionError> {
        let dict = StarlarkDictRef::from_value(*value).ok_or_else(|| {
            ExpressionError::StarlarkConversionError(
                "AssistantMessageExpression: expected dict".into(),
            )
        })?;
        let mut content = WithExpression::Value(None);
        let mut refusal = WithExpression::Value(None);
        let mut tool_calls = WithExpression::Value(None);
        let mut reasoning = WithExpression::Value(None);
        for (k, v) in dict.iter() {
            let key = <&str as UnpackValue>::unpack_value(k)
                .map_err(|e| {
                    ExpressionError::StarlarkConversionError(e.to_string())
                })?
                .ok_or_else(|| {
                    ExpressionError::StarlarkConversionError(
                        "AssistantMessageExpression: expected string key"
                            .into(),
                    )
                })?;
            match key {
                "content" => {
                    content = WithExpression::Value(if v.is_none() {
                        None
                    } else {
                        Some(RichContentExpression::from_starlark_value(&v)?)
                    });
                }
                "refusal" => {
                    refusal = WithExpression::Value(if v.is_none() {
                        None
                    } else {
                        Some(String::from_starlark_value(&v)?)
                    });
                }
                "tool_calls" => {
                    tool_calls = WithExpression::Value(if v.is_none() {
                        None
                    } else {
                        Some(Vec::<WithExpression<AssistantToolCallExpression>>::from_starlark_value(&v)?)
                    });
                }
                "reasoning" => {
                    reasoning = WithExpression::Value(if v.is_none() {
                        None
                    } else {
                        Some(String::from_starlark_value(&v)?)
                    });
                }
                _ => {}
            }
        }
        Ok(AssistantMessageExpression {
            content,
            refusal,
            tool_calls,
            reasoning,
        })
    }
}

/// A tool call made by the assistant.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "agent.completions.message.AssistantToolCall")]
pub enum AssistantToolCall {
    /// A function call with an ID and function details.
    Function {
        /// The unique ID of this tool call.
        id: String,
        /// The function being called.
        function: AssistantToolCallFunction,
    },
}

impl AssistantToolCall {
    /// Returns `true` if all fields are empty.
    pub fn is_empty(&self) -> bool {
        match self {
            AssistantToolCall::Function { id, function } => {
                id.is_empty() && function.is_empty()
            }
        }
    }
}

impl From<AssistantToolCallDelta> for AssistantToolCall {
    fn from(tc: AssistantToolCallDelta) -> Self {
        AssistantToolCall::Function {
            id: tc.id.unwrap_or_default(),
            function: tc.function.unwrap_or_default().into(),
        }
    }
}

impl FromStarlarkValue for AssistantToolCall {
    fn from_starlark_value(
        value: &StarlarkValue,
    ) -> Result<Self, ExpressionError> {
        let dict = StarlarkDictRef::from_value(*value).ok_or_else(|| {
            ExpressionError::StarlarkConversionError(
                "AssistantToolCall: expected dict".into(),
            )
        })?;
        let mut id = None;
        let mut function = None;
        for (k, v) in dict.iter() {
            let key = <&str as UnpackValue>::unpack_value(k)
                .map_err(|e| {
                    ExpressionError::StarlarkConversionError(e.to_string())
                })?
                .ok_or_else(|| {
                    ExpressionError::StarlarkConversionError(
                        "AssistantToolCall: expected string key".into(),
                    )
                })?;
            match key {
                "id" => id = Some(String::from_starlark_value(&v)?),
                "function" => {
                    function = Some(
                        AssistantToolCallFunction::from_starlark_value(&v)?,
                    )
                }
                _ => {}
            }
            if id.is_some() && function.is_some() {
                break;
            }
        }
        Ok(AssistantToolCall::Function {
            id: id.unwrap_or_default(),
            function: function.ok_or_else(|| {
                ExpressionError::StarlarkConversionError(
                    "AssistantToolCall: missing function".into(),
                )
            })?,
        })
    }
}

/// Expression variant of [`AssistantToolCall`] for dynamic content.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "agent.completions.message.AssistantToolCallExpression")]
pub enum AssistantToolCallExpression {
    /// A function call expression.
    Function {
        /// The tool call ID expression.
        id: functions::expression::WithExpression<String>,
        /// The function expression.
        function: functions::expression::WithExpression<
            AssistantToolCallFunctionExpression,
        >,
    },
}

impl AssistantToolCallExpression {
    /// Compiles the expression into a concrete [`AssistantToolCall`].
    pub fn compile(
        self,
        params: &functions::expression::Params,
    ) -> Result<AssistantToolCall, functions::expression::ExpressionError> {
        match self {
            AssistantToolCallExpression::Function { id, function } => {
                let id = id.compile_one(params)?;
                let function = function.compile_one(params)?.compile(params)?;
                Ok(AssistantToolCall::Function { id, function })
            }
        }
    }
}

impl FromStarlarkValue for AssistantToolCallExpression {
    fn from_starlark_value(
        value: &StarlarkValue,
    ) -> Result<Self, ExpressionError> {
        let call = AssistantToolCall::from_starlark_value(value)?;
        match call {
            AssistantToolCall::Function { id, function } => {
                Ok(AssistantToolCallExpression::Function {
                    id: WithExpression::Value(id),
                    function: WithExpression::Value(
                        AssistantToolCallFunctionExpression {
                            name: WithExpression::Value(function.name),
                            arguments: WithExpression::Value(
                                function.arguments,
                            ),
                        },
                    ),
                })
            }
        }
    }
}

/// Details of a function call made by the assistant.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.completions.message.AssistantToolCallFunction")]
pub struct AssistantToolCallFunction {
    /// The name of the function to call.
    pub name: String,
    /// The arguments to pass to the function, as a JSON string.
    pub arguments: String,
}

impl AssistantToolCallFunction {
    /// Returns `true` if both name and arguments are empty.
    pub fn is_empty(&self) -> bool {
        self.name.is_empty() && self.arguments.is_empty()
    }
}

impl From<AssistantToolCallFunctionDelta> for AssistantToolCallFunction {
    fn from(f: AssistantToolCallFunctionDelta) -> Self {
        Self {
            name: f.name.unwrap_or_default(),
            arguments: f.arguments.unwrap_or_default(),
        }
    }
}

impl FromStarlarkValue for AssistantToolCallFunction {
    fn from_starlark_value(
        value: &StarlarkValue,
    ) -> Result<Self, ExpressionError> {
        let dict = StarlarkDictRef::from_value(*value).ok_or_else(|| {
            ExpressionError::StarlarkConversionError(
                "AssistantToolCallFunction: expected dict".into(),
            )
        })?;
        let mut name = None;
        let mut arguments = None;
        for (k, v) in dict.iter() {
            let key = <&str as UnpackValue>::unpack_value(k)
                .map_err(|e| {
                    ExpressionError::StarlarkConversionError(e.to_string())
                })?
                .ok_or_else(|| {
                    ExpressionError::StarlarkConversionError(
                        "AssistantToolCallFunction: expected string key".into(),
                    )
                })?;
            match key {
                "name" => name = Some(String::from_starlark_value(&v)?),
                "arguments" => {
                    arguments = Some(String::from_starlark_value(&v)?)
                }
                _ => {}
            }
            if name.is_some() && arguments.is_some() {
                break;
            }
        }
        Ok(AssistantToolCallFunction {
            name: name.ok_or_else(|| {
                ExpressionError::StarlarkConversionError(
                    "AssistantToolCallFunction: missing name".into(),
                )
            })?,
            arguments: arguments.ok_or_else(|| {
                ExpressionError::StarlarkConversionError(
                    "AssistantToolCallFunction: missing arguments".into(),
                )
            })?,
        })
    }
}

/// A tool call delta in a streaming response.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.completions.message.AssistantToolCallDelta")]
pub struct AssistantToolCallDelta {
    /// The index of this tool call.
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub index: u64,
    /// The type of tool call (always "function").
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub r#type: Option<AssistantToolCallType>,
    /// The unique ID of this tool call.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub id: Option<String>,
    /// The function call details.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub function: Option<AssistantToolCallFunctionDelta>,
}

impl AssistantToolCallDelta {
    /// Accumulates another tool call into this one.
    pub fn push(&mut self, other: &AssistantToolCallDelta) {
        if self.r#type.is_none() {
            self.r#type = other.r#type;
        }
        if self.id.is_none() {
            self.id = other.id.clone();
        }
        match (&mut self.function, &other.function) {
            (Some(self_function), Some(other_function)) => {
                self_function.push(other_function);
            }
            (None, Some(other_function)) => {
                self.function = Some(other_function.clone());
            }
            _ => {}
        }
    }
}

/// The type of tool call.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Serialize,
    Deserialize,
    Default,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.completions.message.AssistantToolCallType")]
pub enum AssistantToolCallType {
    /// A function call.
    #[serde(rename = "function")]
    #[default]
    Function,
}

/// Function call details in a streaming tool call.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Default,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.completions.message.AssistantToolCallFunctionDelta")]
pub struct AssistantToolCallFunctionDelta {
    /// The function name (only present in the first delta).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub name: Option<String>,
    /// The arguments being streamed (accumulated across deltas).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub arguments: Option<String>,
}

impl AssistantToolCallFunctionDelta {
    /// Accumulates another function call delta into this one.
    pub fn push(&mut self, other: &AssistantToolCallFunctionDelta) {
        if self.name.is_none() {
            self.name = other.name.clone();
        }
        match (&mut self.arguments, &other.arguments) {
            (Some(self_arguments), Some(other_arguments)) => {
                self_arguments.push_str(other_arguments);
            }
            (None, Some(other_arguments)) => {
                self.arguments = Some(other_arguments.clone());
            }
            _ => {}
        }
    }
}

/// Expression variant of [`AssistantToolCallFunction`] for dynamic content.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(
    rename = "agent.completions.message.AssistantToolCallFunctionExpression"
)]
pub struct AssistantToolCallFunctionExpression {
    /// The function name expression.
    pub name: functions::expression::WithExpression<String>,
    /// The arguments expression.
    pub arguments: functions::expression::WithExpression<String>,
}

impl AssistantToolCallFunctionExpression {
    /// Compiles the expression into a concrete [`AssistantToolCallFunction`].
    pub fn compile(
        self,
        params: &functions::expression::Params,
    ) -> Result<AssistantToolCallFunction, functions::expression::ExpressionError>
    {
        let name = self.name.compile_one(params)?;
        let arguments = self.arguments.compile_one(params)?;
        Ok(AssistantToolCallFunction { name, arguments })
    }
}

impl FromStarlarkValue for AssistantToolCallFunctionExpression {
    fn from_starlark_value(
        value: &StarlarkValue,
    ) -> Result<Self, ExpressionError> {
        let f = AssistantToolCallFunction::from_starlark_value(value)?;
        Ok(AssistantToolCallFunctionExpression {
            name: WithExpression::Value(f.name),
            arguments: WithExpression::Value(f.arguments),
        })
    }
}

crate::functions::expression::impl_from_special_unsupported!(
    AssistantToolCallExpression,
    AssistantToolCallFunctionExpression,
);

impl crate::functions::expression::FromSpecial
    for Vec<
        crate::functions::expression::WithExpression<
            AssistantToolCallExpression,
        >,
    >
{
    fn from_special(
        _special: &crate::functions::expression::Special,
        _params: &crate::functions::expression::Params,
    ) -> Result<Self, crate::functions::expression::ExpressionError> {
        Err(crate::functions::expression::ExpressionError::UnsupportedSpecial)
    }
}
