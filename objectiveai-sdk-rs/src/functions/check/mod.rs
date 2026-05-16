//! Shared quality-check helpers for function definitions.
//!
//! - [`example_inputs`] — RNG-based example input generation from an `InputSchema`
//! - [`check_vector_fields`] — validates output_length, input_split, and input_merge
//! - [`check_scalar_fields`] — validates scalar function input_schema

mod check_description;
mod check_input_schema;
mod check_modalities;
mod check_output_expression;
mod check_scalar_fields;
mod check_vector_fields;
mod compile_and_validate;
pub mod example_inputs;

pub(crate) use check_description::check_description;
pub(crate) use check_input_schema::check_input_schema;
pub(crate) use check_modalities::{
    ModalityFlags, check_modality_coverage, collect_schema_modalities,
    collect_task_modalities,
};
pub(crate) use check_output_expression::{
    ScalarOutputShape, VectorOutputShape, check_scalar_distribution,
    check_vector_distribution,
};
pub use check_scalar_fields::{ScalarFieldsValidation, check_scalar_fields};
pub use check_vector_fields::{VectorFieldsValidation, check_vector_fields};
pub(crate) use check_vector_fields::{check_vector_fields_for_input, random_subsets};
pub(crate) use compile_and_validate::{
    compile_and_validate_one_input, extract_task_input, extract_task_input_value,
};
