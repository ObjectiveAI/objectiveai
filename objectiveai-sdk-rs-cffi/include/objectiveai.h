/* ObjectiveAI C FFI bindings.
 *
 * All functions follow the same convention:
 *   - Input:  JSON bytes as (const uint8_t*, size_t)
 *   - Output: JSON bytes written to (uint8_t**, size_t*)
 *   - Return: 0 on success, -1 on error (error message in output)
 *   - Memory: Output must be freed with objectiveai_free()
 *
 * Generated from objectiveai-sdk-rs-cffi. Do not edit manually.
 */

#ifndef OBJECTIVEAI_H
#define OBJECTIVEAI_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Memory management */
void objectiveai_free(uint8_t *ptr, size_t len);

/* Validation & ID computation */
int32_t objectiveai_validate_agent(
    const uint8_t *json_in, size_t json_in_len,
    uint8_t **json_out, size_t *json_out_len);

int32_t objectiveai_validate_swarm(
    const uint8_t *json_in, size_t json_in_len,
    uint8_t **json_out, size_t *json_out_len);

int32_t objectiveai_prompt_id(
    const uint8_t *json_in, size_t json_in_len,
    uint8_t **json_out, size_t *json_out_len);

int32_t objectiveai_vector_response_id(
    const uint8_t *json_in, size_t json_in_len,
    uint8_t **json_out, size_t *json_out_len);

/* Function input validation.
 * Returns: 1=valid, 0=invalid, 2=not applicable (inline), -1=error */
int32_t objectiveai_validate_function_input(
    const uint8_t *function_in, size_t function_in_len,
    const uint8_t *input_in, size_t input_in_len,
    uint8_t **json_out, size_t *json_out_len);

/* Function task compilation (two-input functions) */
int32_t objectiveai_compile_function_tasks(
    const uint8_t *function_in, size_t function_in_len,
    const uint8_t *input_in, size_t input_in_len,
    uint8_t **json_out, size_t *json_out_len);

int32_t objectiveai_compile_function_output_length(
    const uint8_t *function_in, size_t function_in_len,
    const uint8_t *input_in, size_t input_in_len,
    uint8_t **json_out, size_t *json_out_len);

int32_t objectiveai_compile_function_input_split(
    const uint8_t *function_in, size_t function_in_len,
    const uint8_t *input_in, size_t input_in_len,
    uint8_t **json_out, size_t *json_out_len);

int32_t objectiveai_compile_function_input_merge(
    const uint8_t *function_in, size_t function_in_len,
    const uint8_t *input_in, size_t input_in_len,
    uint8_t **json_out, size_t *json_out_len);

/* Vector/scalar field validation */
int32_t objectiveai_check_vector_fields(
    const uint8_t *json_in, size_t json_in_len,
    uint8_t **json_out, size_t *json_out_len);

int32_t objectiveai_check_scalar_fields(
    const uint8_t *json_in, size_t json_in_len,
    uint8_t **json_out, size_t *json_out_len);

/* Alpha function validation */
int32_t objectiveai_alpha_check_leaf_scalar_function(
    const uint8_t *json_in, size_t json_in_len,
    uint8_t **json_out, size_t *json_out_len);

int32_t objectiveai_alpha_check_branch_scalar_function(
    const uint8_t *function_in, size_t function_in_len,
    const uint8_t *children_in, size_t children_in_len,
    uint8_t **json_out, size_t *json_out_len);

int32_t objectiveai_alpha_check_leaf_vector_function(
    const uint8_t *json_in, size_t json_in_len,
    uint8_t **json_out, size_t *json_out_len);

int32_t objectiveai_alpha_check_branch_vector_function(
    const uint8_t *function_in, size_t function_in_len,
    const uint8_t *children_in, size_t children_in_len,
    uint8_t **json_out, size_t *json_out_len);

/* Streaming chunk merging (two-input functions) */
int32_t objectiveai_agent_completion_chunk_merged(
    const uint8_t *a_in, size_t a_in_len,
    const uint8_t *b_in, size_t b_in_len,
    uint8_t **json_out, size_t *json_out_len);

int32_t objectiveai_vector_completion_chunk_merged(
    const uint8_t *a_in, size_t a_in_len,
    const uint8_t *b_in, size_t b_in_len,
    uint8_t **json_out, size_t *json_out_len);

int32_t objectiveai_function_execution_chunk_merged(
    const uint8_t *a_in, size_t a_in_len,
    const uint8_t *b_in, size_t b_in_len,
    uint8_t **json_out, size_t *json_out_len);

int32_t objectiveai_function_profile_computation_chunk_merged(
    const uint8_t *a_in, size_t a_in_len,
    const uint8_t *b_in, size_t b_in_len,
    uint8_t **json_out, size_t *json_out_len);

/* Streaming chunk normalization */
int32_t objectiveai_agent_completion_chunk_normalized(
    const uint8_t *json_in, size_t json_in_len,
    uint8_t **json_out, size_t *json_out_len);

int32_t objectiveai_vector_completion_chunk_normalized(
    const uint8_t *json_in, size_t json_in_len,
    uint8_t **json_out, size_t *json_out_len);

int32_t objectiveai_function_execution_chunk_normalized(
    const uint8_t *json_in, size_t json_in_len,
    uint8_t **json_out, size_t *json_out_len);

int32_t objectiveai_function_profile_computation_chunk_normalized(
    const uint8_t *json_in, size_t json_in_len,
    uint8_t **json_out, size_t *json_out_len);

/* Streaming chunk to unary conversion */
int32_t objectiveai_agent_completion_chunk_to_unary(
    const uint8_t *json_in, size_t json_in_len,
    uint8_t **json_out, size_t *json_out_len);

int32_t objectiveai_vector_completion_chunk_to_unary(
    const uint8_t *json_in, size_t json_in_len,
    uint8_t **json_out, size_t *json_out_len);

int32_t objectiveai_function_execution_chunk_to_unary(
    const uint8_t *json_in, size_t json_in_len,
    uint8_t **json_out, size_t *json_out_len);

int32_t objectiveai_function_profile_computation_chunk_to_unary(
    const uint8_t *json_in, size_t json_in_len,
    uint8_t **json_out, size_t *json_out_len);

/* Generate arbitrary chunks (for testing) */
int32_t objectiveai_generate_agent_completion_chunk(
    int32_t has_seed, int64_t seed,
    uint8_t **json_out, size_t *json_out_len);

int32_t objectiveai_generate_vector_completion_chunk(
    int32_t has_seed, int64_t seed,
    uint8_t **json_out, size_t *json_out_len);

int32_t objectiveai_generate_function_execution_chunk(
    int32_t has_seed, int64_t seed,
    uint8_t **json_out, size_t *json_out_len);

int32_t objectiveai_generate_function_profile_computation_chunk(
    int32_t has_seed, int64_t seed,
    uint8_t **json_out, size_t *json_out_len);

#ifdef __cplusplus
}
#endif

#endif /* OBJECTIVEAI_H */
