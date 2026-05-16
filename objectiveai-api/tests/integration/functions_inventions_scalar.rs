//! Scalar leaf + branch invention snapshot tests, plus the
//! pre-provided-schema variants for both. ~200 tests.

use crate::{invention_test_10x, invention_test_10x_schema};
use objectiveai_sdk::functions::inventions::state::{
    AlphaScalarBranchState, AlphaScalarLeafState,
};

// ---------------------------------------------------------------------------
// Scalar Leaf snapshot tests
// ---------------------------------------------------------------------------

invention_test_10x!(test_scalar_leaf_s42,
    AlphaScalarLeaf, AlphaScalarLeafState,
    "sl-default", 0, 3, 5, 3, 5, 42,
    "scalar_leaf_s42");

invention_test_10x!(test_scalar_leaf_s7,
    AlphaScalarLeaf, AlphaScalarLeafState,
    "sl-min-1", 0, 1, 1, 1, 1, 7,
    "scalar_leaf_s7");

invention_test_10x!(test_scalar_leaf_s1337,
    AlphaScalarLeaf, AlphaScalarLeafState,
    "sl-narrow", 0, 2, 3, 2, 3, 1337,
    "scalar_leaf_s1337");

invention_test_10x!(test_scalar_leaf_s999,
    AlphaScalarLeaf, AlphaScalarLeafState,
    "sl-wide-10", 0, 10, 10, 10, 10, 999,
    "scalar_leaf_s999");

invention_test_10x!(test_scalar_leaf_s314,
    AlphaScalarLeaf, AlphaScalarLeafState,
    "sl-asym", 0, 1, 2, 7, 10, 314,
    "scalar_leaf_s314");

invention_test_10x!(test_scalar_leaf_s8675309,
    AlphaScalarLeaf, AlphaScalarLeafState,
    "sl-range", 0, 1, 10, 1, 8, 8675309,
    "scalar_leaf_s8675309");

// ---------------------------------------------------------------------------
// Scalar Branch snapshot tests
// ---------------------------------------------------------------------------

invention_test_10x!(test_scalar_branch_s42,
    AlphaScalarBranch, AlphaScalarBranchState,
    "sb-default", 1, 3, 5, 3, 5, 42,
    "scalar_branch_s42");

invention_test_10x!(test_scalar_branch_s13,
    AlphaScalarBranch, AlphaScalarBranchState,
    "sb-min-1", 1, 1, 1, 1, 1, 13,
    "scalar_branch_s13");

invention_test_10x!(test_scalar_branch_s2718,
    AlphaScalarBranch, AlphaScalarBranchState,
    "sb-narrow", 1, 2, 2, 2, 2, 2718,
    "scalar_branch_s2718");

invention_test_10x!(test_scalar_branch_s77777,
    AlphaScalarBranch, AlphaScalarBranchState,
    "sb-wide-d2", 2, 10, 10, 10, 10, 77777,
    "scalar_branch_s77777");

invention_test_10x!(test_scalar_branch_s555,
    AlphaScalarBranch, AlphaScalarBranchState,
    "sb-asym", 1, 8, 10, 1, 2, 555,
    "scalar_branch_s555");

invention_test_10x!(test_scalar_branch_s161803,
    AlphaScalarBranch, AlphaScalarBranchState,
    "sb-deep", 3, 2, 3, 2, 3, 161803,
    "scalar_branch_s161803");

// ---------------------------------------------------------------------------
// Pre-provided schema tests — Scalar
// ---------------------------------------------------------------------------

invention_test_10x_schema!(test_scalar_leaf_schema_anyof,
    AlphaScalarLeaf, AlphaScalarLeafState,
    "sl-anyof", 0, 3, 5, 3, 5, 50001,
    "scalar_leaf_schema_anyof",
    crate::common::inventions::scalar_schema_anyof_chaos());

invention_test_10x_schema!(test_scalar_leaf_schema_deep,
    AlphaScalarLeaf, AlphaScalarLeafState,
    "sl-deep-media", 0, 2, 4, 2, 4, 60002,
    "scalar_leaf_schema_deep",
    crate::common::inventions::scalar_schema_deep_media());

invention_test_10x_schema!(test_scalar_leaf_schema_arraymad,
    AlphaScalarLeaf, AlphaScalarLeafState,
    "sl-arr-mad", 0, 1, 3, 1, 3, 70003,
    "scalar_leaf_schema_arraymad",
    crate::common::inventions::scalar_schema_array_madness());

invention_test_10x_schema!(test_scalar_leaf_schema_kitchen,
    AlphaScalarLeaf, AlphaScalarLeafState,
    "sl-kitchen", 0, 3, 5, 3, 5, 80004,
    "scalar_leaf_schema_kitchen",
    crate::common::inventions::scalar_schema_kitchen_sink());

invention_test_10x_schema!(test_scalar_branch_schema_anyof,
    AlphaScalarBranch, AlphaScalarBranchState,
    "sb-anyof", 1, 2, 3, 2, 3, 50005,
    "scalar_branch_schema_anyof",
    crate::common::inventions::scalar_schema_anyof_chaos());

invention_test_10x_schema!(test_scalar_branch_schema_deep,
    AlphaScalarBranch, AlphaScalarBranchState,
    "sb-deep-media", 1, 1, 2, 1, 2, 60006,
    "scalar_branch_schema_deep",
    crate::common::inventions::scalar_schema_deep_media());

invention_test_10x_schema!(test_scalar_branch_schema_arraymad,
    AlphaScalarBranch, AlphaScalarBranchState,
    "sb-arr-mad", 1, 2, 4, 2, 4, 70007,
    "scalar_branch_schema_arraymad",
    crate::common::inventions::scalar_schema_array_madness());

invention_test_10x_schema!(test_scalar_branch_schema_kitchen,
    AlphaScalarBranch, AlphaScalarBranchState,
    "sb-kitchen", 2, 2, 3, 2, 3, 80008,
    "scalar_branch_schema_kitchen",
    crate::common::inventions::scalar_schema_kitchen_sink());
