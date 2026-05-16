//! Vector leaf + branch invention snapshot tests, plus the
//! pre-provided-schema variants for both. ~200 tests.

use crate::{invention_test_10x, invention_test_10x_schema};
use objectiveai_sdk::functions::inventions::state::{
    AlphaVectorBranchState, AlphaVectorLeafState,
};

// ---------------------------------------------------------------------------
// Vector Leaf snapshot tests
// ---------------------------------------------------------------------------

invention_test_10x!(test_vector_leaf_s42,
    AlphaVectorLeaf, AlphaVectorLeafState,
    "vl-default", 0, 3, 5, 3, 5, 42,
    "vector_leaf_s42");

invention_test_10x!(test_vector_leaf_s23,
    AlphaVectorLeaf, AlphaVectorLeafState,
    "vl-min-1", 0, 1, 1, 1, 1, 23,
    "vector_leaf_s23");

invention_test_10x!(test_vector_leaf_s404,
    AlphaVectorLeaf, AlphaVectorLeafState,
    "vl-narrow", 0, 2, 2, 2, 2, 404,
    "vector_leaf_s404");

invention_test_10x!(test_vector_leaf_s31415,
    AlphaVectorLeaf, AlphaVectorLeafState,
    "vl-wide-10", 0, 10, 10, 10, 10, 31415,
    "vector_leaf_s31415");

invention_test_10x!(test_vector_leaf_s65536,
    AlphaVectorLeaf, AlphaVectorLeafState,
    "vl-asym", 0, 2, 3, 6, 10, 65536,
    "vector_leaf_s65536");

invention_test_10x!(test_vector_leaf_s271828,
    AlphaVectorLeaf, AlphaVectorLeafState,
    "vl-range", 0, 1, 10, 1, 10, 271828,
    "vector_leaf_s271828");

// ---------------------------------------------------------------------------
// Vector Branch snapshot tests
// ---------------------------------------------------------------------------

invention_test_10x!(test_vector_branch_s42,
    AlphaVectorBranch, AlphaVectorBranchState,
    "vb-default", 1, 3, 5, 3, 5, 42,
    "vector_branch_s42");

invention_test_10x!(test_vector_branch_s71,
    AlphaVectorBranch, AlphaVectorBranchState,
    "vb-min-1", 1, 1, 1, 1, 1, 71,
    "vector_branch_s71");

invention_test_10x!(test_vector_branch_s12345,
    AlphaVectorBranch, AlphaVectorBranchState,
    "vb-narrow", 1, 2, 2, 2, 2, 12345,
    "vector_branch_s12345");

invention_test_10x!(test_vector_branch_s90210,
    AlphaVectorBranch, AlphaVectorBranchState,
    "vb-wide-d2", 2, 10, 10, 10, 10, 90210,
    "vector_branch_s90210");

invention_test_10x!(test_vector_branch_s1984,
    AlphaVectorBranch, AlphaVectorBranchState,
    "vb-asym", 1, 1, 2, 8, 10, 1984,
    "vector_branch_s1984");

invention_test_10x!(test_vector_branch_s2025,
    AlphaVectorBranch, AlphaVectorBranchState,
    "vb-deep", 3, 2, 4, 2, 4, 2025,
    "vector_branch_s2025");

// ---------------------------------------------------------------------------
// Pre-provided schema tests — Vector
// ---------------------------------------------------------------------------

invention_test_10x_schema!(test_vector_leaf_schema_multimedia,
    AlphaVectorLeaf, AlphaVectorLeafState,
    "vl-multimedia", 0, 1, 3, 1, 3, 50009,
    "vector_leaf_schema_multimedia",
    crate::common::inventions::vector_schema_multimedia_ranking());

invention_test_10x_schema!(test_vector_leaf_schema_chaos,
    AlphaVectorLeaf, AlphaVectorLeafState,
    "vl-chaos", 0, 1, 2, 1, 2, 60010,
    "vector_leaf_schema_chaos",
    crate::common::inventions::vector_schema_nested_chaos());

invention_test_10x_schema!(test_vector_leaf_schema_richctx,
    AlphaVectorLeaf, AlphaVectorLeafState,
    "vl-richctx", 0, 1, 2, 1, 2, 70011,
    "vector_leaf_schema_richctx",
    crate::common::inventions::vector_schema_rich_context());

invention_test_10x_schema!(test_vector_leaf_schema_noctx,
    AlphaVectorLeaf, AlphaVectorLeafState,
    "vl-noctx", 0, 1, 2, 1, 2, 80012,
    "vector_leaf_schema_noctx",
    crate::common::inventions::vector_schema_no_context_deep_items());

invention_test_10x_schema!(test_vector_branch_schema_multimedia,
    AlphaVectorBranch, AlphaVectorBranchState,
    "vb-multimedia", 1, 1, 2, 1, 2, 50013,
    "vector_branch_schema_multimedia",
    crate::common::inventions::vector_schema_multimedia_ranking());

invention_test_10x_schema!(test_vector_branch_schema_chaos,
    AlphaVectorBranch, AlphaVectorBranchState,
    "vb-chaos", 1, 1, 2, 1, 2, 60014,
    "vector_branch_schema_chaos",
    crate::common::inventions::vector_schema_nested_chaos());

invention_test_10x_schema!(test_vector_branch_schema_richctx,
    AlphaVectorBranch, AlphaVectorBranchState,
    "vb-richctx", 1, 1, 2, 1, 2, 70015,
    "vector_branch_schema_richctx",
    crate::common::inventions::vector_schema_rich_context());

invention_test_10x_schema!(test_vector_branch_schema_noctx,
    AlphaVectorBranch, AlphaVectorBranchState,
    "vb-noctx", 1, 1, 2, 1, 2, 80016,
    "vector_branch_schema_noctx",
    crate::common::inventions::vector_schema_no_context_deep_items());
