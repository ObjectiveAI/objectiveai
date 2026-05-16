use indexmap::IndexMap;
use serde::Deserialize;

pub trait JsonSchema {
    fn json_schema() -> serde_json::Map<String, serde_json::Value>;
    fn indexmap_json_schema() -> IndexMap<String, serde_json::Value> {
        Self::json_schema().into_iter().collect()
    }
}

pub struct EmptyObjectJsonSchema;

impl JsonSchema for EmptyObjectJsonSchema {
    fn json_schema() -> serde_json::Map<String, serde_json::Value> {
        let mut map = serde_json::Map::new();
        map.insert(
            "type".to_string(),
            serde_json::Value::String("object".to_string()),
        );
        map.insert(
            "additionalProperties".to_string(),
            serde_json::Value::Bool(false),
        );
        map
    }
}

pub struct AnyObjectJsonSchema;

impl JsonSchema for AnyObjectJsonSchema {
    fn json_schema() -> serde_json::Map<String, serde_json::Value> {
        let mut map = serde_json::Map::new();
        map.insert(
            "type".to_string(),
            serde_json::Value::String("object".to_string()),
        );
        map
    }
}

/// Helper to build a single-string-property schema with a description.
fn string_property_schema(prop_name: &str, description: &str) -> serde_json::Map<String, serde_json::Value> {
    let mut prop = serde_json::Map::with_capacity(2);
    prop.insert("type".to_string(), serde_json::Value::String("string".to_string()));
    prop.insert("description".to_string(), serde_json::Value::String(description.to_string()));

    let mut properties = serde_json::Map::with_capacity(1);
    properties.insert(prop_name.to_string(), serde_json::Value::Object(prop));

    let mut map = serde_json::Map::with_capacity(4);
    map.insert("type".to_string(), serde_json::Value::String("object".to_string()));
    map.insert("properties".to_string(), serde_json::Value::Object(properties));
    map.insert("required".to_string(), serde_json::Value::Array(vec![serde_json::Value::String(prop_name.to_string())]));
    map.insert("additionalProperties".to_string(), serde_json::Value::Bool(false));
    map
}

/// Schema for WriteInputSchema on scalar functions.
/// Takes a `schema` property: a JSON string conforming to `functions.expression.ObjectInputSchema`.
#[derive(Deserialize, schemars::JsonSchema)]
#[schemars(rename = "functions.inventions.ScalarInputSchemaObject")]
pub struct ScalarInputSchemaObject {
    pub schema: String,
}

impl JsonSchema for ScalarInputSchemaObject {
    fn json_schema() -> serde_json::Map<String, serde_json::Value> {
        string_property_schema("schema", "A JSON string conforming to the functions.expression.ObjectInputSchema schema. Use the ReadObjectInputSchemaSchema tool to see the schema definition.")
    }
}

/// Schema for WriteInputSchema on vector functions.
/// Takes a `schema` property: a JSON string conforming to `functions.alpha_vector.expression.VectorFunctionInputSchema`.
#[derive(Deserialize, schemars::JsonSchema)]
#[schemars(rename = "functions.inventions.VectorInputSchemaObject")]
pub struct VectorInputSchemaObject {
    pub schema: String,
}

impl JsonSchema for VectorInputSchemaObject {
    fn json_schema() -> serde_json::Map<String, serde_json::Value> {
        string_property_schema("schema", "A JSON string conforming to the functions.alpha_vector.expression.VectorFunctionInputSchema schema. Use the Readfunctions_alpha_vector_expression_VectorFunctionInputSchemaSchema tool to see the schema definition.")
    }
}

/// Schema for AppendTask on scalar leaf functions.
/// Takes a `task` property: a JSON string conforming to `functions.alpha_scalar.LeafTaskExpression`.
#[derive(Deserialize, schemars::JsonSchema)]
#[schemars(rename = "functions.inventions.ScalarLeafTaskObject")]
pub struct ScalarLeafTaskObject {
    pub task: String,
}

impl JsonSchema for ScalarLeafTaskObject {
    fn json_schema() -> serde_json::Map<String, serde_json::Value> {
        string_property_schema("task", "A JSON string conforming to the functions.alpha_scalar.LeafTaskExpression schema. Use the Readfunctions_alpha_scalar_LeafTaskExpressionSchema tool to see the schema definition.")
    }
}

/// Schema for AppendTask on scalar branch functions.
/// Takes a `task` property: a JSON string conforming to `functions.alpha_scalar.PartialPlaceholderBranchTaskExpression`.
#[derive(Deserialize, schemars::JsonSchema)]
#[schemars(rename = "functions.inventions.ScalarBranchTaskObject")]
pub struct ScalarBranchTaskObject {
    pub task: String,
}

impl JsonSchema for ScalarBranchTaskObject {
    fn json_schema() -> serde_json::Map<String, serde_json::Value> {
        string_property_schema("task", "A JSON string conforming to the functions.alpha_scalar.PartialPlaceholderBranchTaskExpression schema. Use the Readfunctions_alpha_scalar_PartialPlaceholderBranchTaskExpressionSchema tool to see the schema definition.")
    }
}

/// Schema for AppendTask on vector leaf functions.
/// Takes a `task` property: a JSON string conforming to `functions.alpha_vector.LeafTaskExpression`.
#[derive(Deserialize, schemars::JsonSchema)]
#[schemars(rename = "functions.inventions.VectorLeafTaskObject")]
pub struct VectorLeafTaskObject {
    pub task: String,
}

impl JsonSchema for VectorLeafTaskObject {
    fn json_schema() -> serde_json::Map<String, serde_json::Value> {
        string_property_schema("task", "A JSON string conforming to the functions.alpha_vector.LeafTaskExpression schema. Use the Readfunctions_alpha_vector_LeafTaskExpressionSchema tool to see the schema definition.")
    }
}

/// Schema for AppendTask on vector branch functions.
/// Takes a `task` property: a JSON string conforming to `functions.alpha_vector.PartialPlaceholderBranchTaskExpression`.
#[derive(Deserialize, schemars::JsonSchema)]
#[schemars(rename = "functions.inventions.VectorBranchTaskObject")]
pub struct VectorBranchTaskObject {
    pub task: String,
}

impl JsonSchema for VectorBranchTaskObject {
    fn json_schema() -> serde_json::Map<String, serde_json::Value> {
        string_property_schema("task", "A JSON string conforming to the functions.alpha_vector.PartialPlaceholderBranchTaskExpression schema. Use the Readfunctions_alpha_vector_PartialPlaceholderBranchTaskExpressionSchema tool to see the schema definition.")
    }
}

#[derive(Deserialize, schemars::JsonSchema)]
#[schemars(rename = "functions.inventions.IndexObject")]
pub struct IndexObject {
    pub index: u64,
}

impl JsonSchema for IndexObject {
    fn json_schema() -> serde_json::Map<String, serde_json::Value> {
        let mut index_prop = serde_json::Map::with_capacity(1);
        index_prop.insert(
            "type".to_string(),
            serde_json::Value::String("integer".to_string()),
        );

        let mut properties = serde_json::Map::with_capacity(1);
        properties.insert(
            "index".to_string(),
            serde_json::Value::Object(index_prop),
        );

        let mut map = serde_json::Map::with_capacity(4);
        map.insert(
            "type".to_string(),
            serde_json::Value::String("object".to_string()),
        );
        map.insert(
            "properties".to_string(),
            serde_json::Value::Object(properties),
        );
        map.insert(
            "required".to_string(),
            serde_json::Value::Array(vec![serde_json::Value::String(
                "index".to_string(),
            )]),
        );
        map.insert(
            "additionalProperties".to_string(),
            serde_json::Value::Bool(false),
        );
        map
    }
}

#[derive(Deserialize, schemars::JsonSchema)]
#[schemars(rename = "functions.inventions.EssayObject")]
pub struct EssayObject {
    pub essay: String,
}

impl JsonSchema for EssayObject {
    fn json_schema() -> serde_json::Map<String, serde_json::Value> {
        let mut essay_prop = serde_json::Map::with_capacity(1);
        essay_prop.insert(
            "type".to_string(),
            serde_json::Value::String("string".to_string()),
        );

        let mut properties = serde_json::Map::with_capacity(1);
        properties.insert(
            "essay".to_string(),
            serde_json::Value::Object(essay_prop),
        );

        let mut map = serde_json::Map::with_capacity(4);
        map.insert(
            "type".to_string(),
            serde_json::Value::String("object".to_string()),
        );
        map.insert(
            "properties".to_string(),
            serde_json::Value::Object(properties),
        );
        map.insert(
            "required".to_string(),
            serde_json::Value::Array(vec![serde_json::Value::String(
                "essay".to_string(),
            )]),
        );
        map.insert(
            "additionalProperties".to_string(),
            serde_json::Value::Bool(false),
        );
        map
    }
}

#[derive(Deserialize, schemars::JsonSchema)]
#[schemars(rename = "functions.inventions.EssayTasksObject")]
pub struct EssayTasksObject {
    pub essay_tasks: String,
}

impl JsonSchema for EssayTasksObject {
    fn json_schema() -> serde_json::Map<String, serde_json::Value> {
        let mut essay_tasks_prop = serde_json::Map::with_capacity(1);
        essay_tasks_prop.insert(
            "type".to_string(),
            serde_json::Value::String("string".to_string()),
        );

        let mut properties = serde_json::Map::with_capacity(1);
        properties.insert(
            "essay_tasks".to_string(),
            serde_json::Value::Object(essay_tasks_prop),
        );

        let mut map = serde_json::Map::with_capacity(4);
        map.insert(
            "type".to_string(),
            serde_json::Value::String("object".to_string()),
        );
        map.insert(
            "properties".to_string(),
            serde_json::Value::Object(properties),
        );
        map.insert(
            "required".to_string(),
            serde_json::Value::Array(vec![serde_json::Value::String(
                "essay_tasks".to_string(),
            )]),
        );
        map.insert(
            "additionalProperties".to_string(),
            serde_json::Value::Bool(false),
        );
        map
    }
}

#[derive(Deserialize, schemars::JsonSchema)]
#[schemars(rename = "functions.inventions.TasksLengthObject")]
pub struct TasksLengthObject {
    pub tasks_length: u64,
}

impl JsonSchema for TasksLengthObject {
    fn json_schema() -> serde_json::Map<String, serde_json::Value> {
        let mut tasks_length_prop = serde_json::Map::with_capacity(1);
        tasks_length_prop.insert(
            "type".to_string(),
            serde_json::Value::String("integer".to_string()),
        );

        let mut properties = serde_json::Map::with_capacity(1);
        properties.insert(
            "tasks_length".to_string(),
            serde_json::Value::Object(tasks_length_prop),
        );

        let mut map = serde_json::Map::with_capacity(4);
        map.insert(
            "type".to_string(),
            serde_json::Value::String("object".to_string()),
        );
        map.insert(
            "properties".to_string(),
            serde_json::Value::Object(properties),
        );
        map.insert(
            "required".to_string(),
            serde_json::Value::Array(vec![serde_json::Value::String(
                "tasks_length".to_string(),
            )]),
        );
        map.insert(
            "additionalProperties".to_string(),
            serde_json::Value::Bool(false),
        );
        map
    }
}

#[derive(Deserialize, schemars::JsonSchema)]
#[schemars(rename = "functions.inventions.DescriptionObject")]
pub struct DescriptionObject {
    pub description: String,
}

impl JsonSchema for DescriptionObject {
    fn json_schema() -> serde_json::Map<String, serde_json::Value> {
        let mut description_prop = serde_json::Map::with_capacity(1);
        description_prop.insert(
            "type".to_string(),
            serde_json::Value::String("string".to_string()),
        );

        let mut properties = serde_json::Map::with_capacity(1);
        properties.insert(
            "description".to_string(),
            serde_json::Value::Object(description_prop),
        );

        let mut map = serde_json::Map::with_capacity(4);
        map.insert(
            "type".to_string(),
            serde_json::Value::String("object".to_string()),
        );
        map.insert(
            "properties".to_string(),
            serde_json::Value::Object(properties),
        );
        map.insert(
            "required".to_string(),
            serde_json::Value::Array(vec![serde_json::Value::String(
                "description".to_string(),
            )]),
        );
        map.insert(
            "additionalProperties".to_string(),
            serde_json::Value::Bool(false),
        );
        map
    }
}
