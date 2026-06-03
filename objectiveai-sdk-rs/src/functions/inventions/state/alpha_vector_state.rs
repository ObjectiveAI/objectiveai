use crate::functions;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.inventions.state.AlphaVectorState")]
pub struct AlphaVectorState {
    #[serde(flatten)]
    pub params: super::Params,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub input_schema:
        Option<functions::alpha_vector::expression::VectorFunctionInputSchema>,
}

impl AlphaVectorState {
    pub(super) fn serialize_into_files(&self) -> super::files::Files {
        super::files::Files {
            parameters_json: serde_json::to_string_pretty(&self.params)
                .unwrap_or_default(),
            function_json: None,
            input_schema_json: self.input_schema.as_ref().map(|s| {
                serde_json::to_string_pretty(&super::InputSchema::Vector {
                    schema: s.clone(),
                })
                .unwrap_or_default()
            }),
            essay_md: None,
            essay_tasks_md: None,
            readme_md: None,
        }
    }

    pub(super) fn deserialize_from_files(
        input_schema: Option<
            functions::alpha_vector::expression::VectorFunctionInputSchema,
        >,
        files: &super::files::Files,
    ) -> Result<Self, super::error::Error> {
        let params: super::Params = {
            let mut de =
                serde_json::Deserializer::from_str(&files.parameters_json);
            serde_path_to_error::deserialize(&mut de).map_err(|e| {
                super::error::Error::Deserialize {
                    file: super::files::Files::PARAMETERS_JSON,
                    source: e,
                }
            })?
        };
        Ok(Self {
            params,
            input_schema,
        })
    }
}
