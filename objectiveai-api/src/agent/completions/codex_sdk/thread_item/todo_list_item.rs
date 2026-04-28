use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TodoListItemType {
    TodoList,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TodoListItem {
    pub id: String,
    pub r#type: TodoListItemType,
    pub items: Vec<super::TodoItem>,
}
