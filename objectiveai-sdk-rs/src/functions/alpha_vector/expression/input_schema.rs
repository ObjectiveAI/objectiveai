use crate::functions;
use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub type ScalarFunctionInputSchema = functions::expression::ObjectInputSchema;

pub mod scalar_function_input_schema {
    use crate::functions;
    pub fn transpile(
        this: super::ScalarFunctionInputSchema,
    ) -> functions::expression::InputSchema {
        functions::expression::InputSchema::Object(this)
    }
}

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
    rename = "functions.alpha_vector.expression.VectorFunctionInputSchema"
)]
pub struct VectorFunctionInputSchema {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub context: Option<functions::expression::ObjectInputSchema>,
    pub items: functions::expression::InputSchema,
}

impl VectorFunctionInputSchema {
    /// Returns which media modalities are present in context and/or items.
    pub fn modalities(&self) -> functions::expression::Modalities {
        let ctx = self
            .context
            .as_ref()
            .map(|c| c.modalities())
            .unwrap_or_default();
        ctx.merge(self.items.modalities())
    }

    pub fn transpile(self) -> functions::expression::InputSchema {
        functions::expression::InputSchema::Object(
            functions::expression::ObjectInputSchema {
                r#type: Default::default(),
                description: None,
                required: Some(if self.context.is_some() {
                    vec!["context".to_string(), "items".to_string()]
                } else {
                    vec!["items".to_string()]
                }),
                properties: {
                    let mut map =
                        IndexMap::with_capacity(if self.context.is_some() {
                            2
                        } else {
                            1
                        });
                    if let Some(context) = self.context {
                        map.insert(
                            "context".to_string(),
                            functions::expression::InputSchema::Object(context),
                        );
                    }
                    map.insert(
                        "items".to_string(),
                        functions::expression::InputSchema::Array(
                            functions::expression::ArrayInputSchema {
                                r#type: Default::default(),
                                description: None,
                                min_items: Some(2),
                                max_items: None,
                                items: Box::new(self.items),
                            },
                        ),
                    );
                    map
                },
            },
        )
    }
}
