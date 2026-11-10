//! The `todo_list` item: the agent's running plan.

use serde::{Deserialize, Serialize};

/// The agent's to-do list: started with the first plan of the turn,
/// updated — the one item that is — with every revision under the
/// same id, and completed at the turn's end with its final state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodoList {
    /// The steps, in order.
    pub items: Vec<TodoItem>,
}

/// One step of the plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodoItem {
    /// The step.
    pub text: String,
    /// Whether it is done.
    pub completed: bool,
}
