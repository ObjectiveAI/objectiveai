mod branch;
mod leaf;

pub use branch::check_alpha_branch_scalar_function;
pub use leaf::check_alpha_leaf_scalar_function;

#[cfg(test)]
mod branch_tests;
#[cfg(test)]
mod leaf_tests;
