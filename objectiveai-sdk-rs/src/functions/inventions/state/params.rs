use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, arbitrary::Arbitrary)]
#[schemars(rename = "functions.inventions.state.Params")]
pub struct Params {
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub depth: u64,
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub min_branch_width: u64,
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub max_branch_width: u64,
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub min_leaf_width: u64,
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub max_leaf_width: u64,
    pub name: String,
    pub spec: String,
}

impl Params {
    /// Validate that the params are usable for invention.
    pub fn validate(&self) -> Result<(), String> {
        if self.min_leaf_width == 0 || self.max_leaf_width == 0 {
            return Err("min_leaf_width and max_leaf_width must be >= 1".to_string());
        }
        if self.min_leaf_width > self.max_leaf_width {
            return Err(format!(
                "min_leaf_width ({}) must be <= max_leaf_width ({})",
                self.min_leaf_width, self.max_leaf_width,
            ));
        }
        if self.depth > 0 {
            if self.min_branch_width == 0 || self.max_branch_width == 0 {
                return Err("min_branch_width and max_branch_width must be >= 1 when depth > 0".to_string());
            }
            if self.min_branch_width > self.max_branch_width {
                return Err(format!(
                    "min_branch_width ({}) must be <= max_branch_width ({})",
                    self.min_branch_width, self.max_branch_width,
                ));
            }
        }
        if self.name.is_empty() {
            return Err("name must not be empty".to_string());
        }
        if self.spec.is_empty() {
            return Err("spec must not be empty".to_string());
        }
        Ok(())
    }
}
