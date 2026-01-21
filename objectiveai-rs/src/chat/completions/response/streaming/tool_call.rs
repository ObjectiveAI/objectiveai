use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub index: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<super::ToolCallType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<ToolCallFunction>,
}

impl ToolCall {
    pub fn push(
        &mut self,
        ToolCall {
            r#type,
            id,
            function,
            ..
        }: &ToolCall,
    ) {
        if self.r#type.is_none() {
            self.r#type = r#type.clone();
        }
        if self.id.is_none() {
            self.id = id.clone();
        }
        match (&mut self.function, &function) {
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub enum ToolCallType {
    #[serde(rename = "function")]
    #[default]
    Function,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolCallFunction {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
}

impl ToolCallFunction {
    pub fn push(&mut self, other: &ToolCallFunction) {
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
