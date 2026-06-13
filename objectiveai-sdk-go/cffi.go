package objectiveai

import (
	"context"
	_ "embed"
	"encoding/binary"
	"encoding/json"
	"errors"
	"fmt"
	"sync"

	"github.com/tetratelabs/wazero"
	"github.com/tetratelabs/wazero/api"
	"github.com/tetratelabs/wazero/imports/wasi_snapshot_preview1"
)

//go:embed lib/objectiveai_cffi.wasm
var cffiWasm []byte

var (
	cffiOnce sync.Once
	cffiMod  api.Module
	cffiMu   sync.Mutex // serializes WASM calls (single-threaded WASM)
	cffiErr  error
)

func cffiInit() {
	cffiOnce.Do(func() {
		ctx := context.Background()
		r := wazero.NewRuntime(ctx)
		wasi_snapshot_preview1.MustInstantiate(ctx, r)
		cffiMod, cffiErr = r.Instantiate(ctx, cffiWasm)
	})
}

func cffiModule() (api.Module, error) {
	cffiInit()
	if cffiErr != nil {
		return nil, fmt.Errorf("objectiveai: failed to initialize WASM runtime: %w", cffiErr)
	}
	return cffiMod, nil
}

// wasmAlloc allocates len bytes in WASM memory and returns the pointer.
func wasmAlloc(mod api.Module, ctx context.Context, size uint32) (uint32, error) {
	fn := mod.ExportedFunction("objectiveai_allocate")
	results, err := fn.Call(ctx, uint64(size))
	if err != nil {
		return 0, err
	}
	return uint32(results[0]), nil
}

// wasmFree frees memory allocated in WASM.
func wasmFree(mod api.Module, ctx context.Context, ptr, size uint32) {
	fn := mod.ExportedFunction("objectiveai_free")
	fn.Call(ctx, uint64(ptr), uint64(size))
}

// wasmWriteBytes writes bytes to WASM memory, returning the pointer.
func wasmWriteBytes(mod api.Module, ctx context.Context, data []byte) (uint32, error) {
	if len(data) == 0 {
		return 0, nil
	}
	ptr, err := wasmAlloc(mod, ctx, uint32(len(data)))
	if err != nil {
		return 0, err
	}
	if !mod.Memory().Write(ptr, data) {
		return 0, fmt.Errorf("objectiveai: memory write out of range")
	}
	return ptr, nil
}

// wasmReadU32 reads a uint32 from WASM memory at the given offset.
func wasmReadU32(mod api.Module, offset uint32) (uint32, bool) {
	buf, ok := mod.Memory().Read(offset, 4)
	if !ok {
		return 0, false
	}
	return binary.LittleEndian.Uint32(buf), true
}

// wasmReadOutput reads the output pointer and length, copies the bytes, and frees the WASM allocation.
func wasmReadOutput(mod api.Module, ctx context.Context, outPtrPtr, outLenPtr uint32) ([]byte, error) {
	outPtr, ok := wasmReadU32(mod, outPtrPtr)
	if !ok {
		return nil, fmt.Errorf("objectiveai: failed to read output pointer")
	}
	outLen, ok := wasmReadU32(mod, outLenPtr)
	if !ok {
		return nil, fmt.Errorf("objectiveai: failed to read output length")
	}
	if outLen == 0 {
		return nil, nil
	}
	outBytes, ok := mod.Memory().Read(outPtr, outLen)
	if !ok {
		return nil, fmt.Errorf("objectiveai: failed to read output bytes")
	}
	result := make([]byte, outLen)
	copy(result, outBytes)
	wasmFree(mod, ctx, outPtr, outLen)
	return result, nil
}

// callWasm1 calls a single-input WASM function (json_in, len, *out, *out_len) -> rc.
func callWasm1(fnName string, jsonIn []byte) ([]byte, int32, error) {
	mod, err := cffiModule()
	if err != nil {
		return nil, -1, err
	}

	cffiMu.Lock()
	defer cffiMu.Unlock()

	ctx := context.Background()

	inPtr, err := wasmWriteBytes(mod, ctx, jsonIn)
	if err != nil {
		return nil, -1, err
	}
	defer wasmFree(mod, ctx, inPtr, uint32(len(jsonIn)))

	outPtrPtr, err := wasmAlloc(mod, ctx, 4)
	if err != nil {
		return nil, -1, err
	}
	defer wasmFree(mod, ctx, outPtrPtr, 4)

	outLenPtr, err := wasmAlloc(mod, ctx, 4)
	if err != nil {
		return nil, -1, err
	}
	defer wasmFree(mod, ctx, outLenPtr, 4)

	fn := mod.ExportedFunction(fnName)
	if fn == nil {
		return nil, -1, fmt.Errorf("objectiveai: WASM function %q not found", fnName)
	}

	results, err := fn.Call(ctx, uint64(inPtr), uint64(len(jsonIn)), uint64(outPtrPtr), uint64(outLenPtr))
	if err != nil {
		return nil, -1, err
	}

	result, err := wasmReadOutput(mod, ctx, outPtrPtr, outLenPtr)
	if err != nil {
		return nil, int32(results[0]), err
	}
	return result, int32(results[0]), nil
}

// callWasm2 calls a two-input WASM function (in1, len1, in2, len2, *out, *out_len) -> rc.
func callWasm2(fnName string, jsonIn1, jsonIn2 []byte) ([]byte, int32, error) {
	mod, err := cffiModule()
	if err != nil {
		return nil, -1, err
	}

	cffiMu.Lock()
	defer cffiMu.Unlock()

	ctx := context.Background()

	in1Ptr, err := wasmWriteBytes(mod, ctx, jsonIn1)
	if err != nil {
		return nil, -1, err
	}
	defer wasmFree(mod, ctx, in1Ptr, uint32(len(jsonIn1)))

	in2Ptr, err := wasmWriteBytes(mod, ctx, jsonIn2)
	if err != nil {
		return nil, -1, err
	}
	defer wasmFree(mod, ctx, in2Ptr, uint32(len(jsonIn2)))

	outPtrPtr, err := wasmAlloc(mod, ctx, 4)
	if err != nil {
		return nil, -1, err
	}
	defer wasmFree(mod, ctx, outPtrPtr, 4)

	outLenPtr, err := wasmAlloc(mod, ctx, 4)
	if err != nil {
		return nil, -1, err
	}
	defer wasmFree(mod, ctx, outLenPtr, 4)

	fn := mod.ExportedFunction(fnName)
	if fn == nil {
		return nil, -1, fmt.Errorf("objectiveai: WASM function %q not found", fnName)
	}

	results, err := fn.Call(ctx,
		uint64(in1Ptr), uint64(len(jsonIn1)),
		uint64(in2Ptr), uint64(len(jsonIn2)),
		uint64(outPtrPtr), uint64(outLenPtr),
	)
	if err != nil {
		return nil, -1, err
	}

	result, err := wasmReadOutput(mod, ctx, outPtrPtr, outLenPtr)
	if err != nil {
		return nil, int32(results[0]), err
	}
	return result, int32(results[0]), nil
}

// callWasmSeed calls a seed-based WASM function (has_seed, seed, *out, *out_len) -> rc.
func callWasmSeed(fnName string, hasSeed bool, seed int64) ([]byte, error) {
	mod, err := cffiModule()
	if err != nil {
		return nil, err
	}

	cffiMu.Lock()
	defer cffiMu.Unlock()

	ctx := context.Background()

	outPtrPtr, err := wasmAlloc(mod, ctx, 4)
	if err != nil {
		return nil, err
	}
	defer wasmFree(mod, ctx, outPtrPtr, 4)

	outLenPtr, err := wasmAlloc(mod, ctx, 4)
	if err != nil {
		return nil, err
	}
	defer wasmFree(mod, ctx, outLenPtr, 4)

	var hs uint64
	if hasSeed {
		hs = 1
	}

	fn := mod.ExportedFunction(fnName)
	if fn == nil {
		return nil, fmt.Errorf("objectiveai: WASM function %q not found", fnName)
	}

	results, err := fn.Call(ctx, hs, uint64(seed), uint64(outPtrPtr), uint64(outLenPtr))
	if err != nil {
		return nil, err
	}

	result, err := wasmReadOutput(mod, ctx, outPtrPtr, outLenPtr)
	if err != nil {
		return nil, err
	}

	if int32(results[0]) != 0 {
		return nil, errors.New(string(result))
	}
	return result, nil
}

// ---------------------------------------------------------------------------
// Typed helper wrappers
// ---------------------------------------------------------------------------

func cffi1[In any, Out any](fnName string, input In) (*Out, error) {
	jsonIn, err := json.Marshal(input)
	if err != nil {
		return nil, err
	}
	result, rc, err := callWasm1(fnName, jsonIn)
	if err != nil {
		return nil, err
	}
	if rc != 0 {
		return nil, errors.New(string(result))
	}
	var out Out
	if err := json.Unmarshal(result, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func cffi1String[In any](fnName string, input In) (string, error) {
	jsonIn, err := json.Marshal(input)
	if err != nil {
		return "", err
	}
	result, rc, err := callWasm1(fnName, jsonIn)
	if err != nil {
		return "", err
	}
	if rc != 0 {
		return "", errors.New(string(result))
	}
	return string(result), nil
}

func cffi1Void[In any](fnName string, input In) error {
	jsonIn, err := json.Marshal(input)
	if err != nil {
		return err
	}
	result, rc, err := callWasm1(fnName, jsonIn)
	if err != nil {
		return err
	}
	if rc != 0 {
		return errors.New(string(result))
	}
	return nil
}

func cffi2[In1 any, In2 any, Out any](fnName string, input1 In1, input2 In2) (*Out, error) {
	jsonIn1, err := json.Marshal(input1)
	if err != nil {
		return nil, err
	}
	jsonIn2, err := json.Marshal(input2)
	if err != nil {
		return nil, err
	}
	result, rc, err := callWasm2(fnName, jsonIn1, jsonIn2)
	if err != nil {
		return nil, err
	}
	if rc != 0 {
		return nil, errors.New(string(result))
	}
	var out Out
	if err := json.Unmarshal(result, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

func cffi2Void[In1 any, In2 any](fnName string, input1 In1, input2 In2) error {
	jsonIn1, err := json.Marshal(input1)
	if err != nil {
		return err
	}
	jsonIn2, err := json.Marshal(input2)
	if err != nil {
		return err
	}
	result, rc, err := callWasm2(fnName, jsonIn1, jsonIn2)
	if err != nil {
		return err
	}
	if rc != 0 {
		return errors.New(string(result))
	}
	return nil
}

func cffiGenerate[Out any](fnName string, hasSeed bool, seed int64) (*Out, error) {
	result, err := callWasmSeed(fnName, hasSeed, seed)
	if err != nil {
		return nil, err
	}
	var out Out
	if err := json.Unmarshal(result, &out); err != nil {
		return nil, err
	}
	return &out, nil
}

// ---------------------------------------------------------------------------
// Memory Management
// ---------------------------------------------------------------------------

// Free is a no-op — all WASM memory is managed internally.
func Free(_ []byte) {}

// ---------------------------------------------------------------------------
// Validation & ID Computation
// ---------------------------------------------------------------------------

// ValidateAgent validates an Agent configuration and computes its content-addressed ID.
func ValidateAgent(agent AgentAgentBase) (*AgentAgent, error) {
	return cffi1[AgentAgentBase, AgentAgent]("objectiveai_validate_agent", agent)
}

// ValidateSwarm validates a Swarm configuration and computes its content-addressed ID.
// Pass nil for remoteAgents if no remote agent definitions are available.
func ValidateSwarm(swarm SwarmSwarmBase, remoteAgents map[string]AgentRemoteAgentBaseWithFallbacks) (*SwarmSwarm, error) {
	return cffi2[SwarmSwarmBase, map[string]AgentRemoteAgentBaseWithFallbacks, SwarmSwarm]("objectiveai_validate_swarm", swarm, remoteAgents)
}

// PromptId computes a content-addressed ID for chat messages.
func PromptId(prompt []AgentCompletionsMessageMessage) (string, error) {
	return cffi1String("objectiveai_prompt_id", prompt)
}

// VectorResponseId computes a content-addressed ID for a vector completion response option.
func VectorResponseId(response AgentCompletionsMessageRichContent) (string, error) {
	return cffi1String("objectiveai_vector_response_id", response)
}

// ---------------------------------------------------------------------------
// Function Input Validation
// ---------------------------------------------------------------------------

// ValidateFunctionInput validates function input against its schema.
// Returns true if valid, false if invalid, nil if not applicable (inline function).
func ValidateFunctionInput(function FunctionsFunction, input FunctionsExpressionInputValue) (*bool, error) {
	jsonFn, err := json.Marshal(function)
	if err != nil {
		return nil, err
	}
	jsonIn, err := json.Marshal(input)
	if err != nil {
		return nil, err
	}
	result, rc, err := callWasm2("objectiveai_validate_function_input", jsonFn, jsonIn)
	if err != nil {
		return nil, err
	}
	switch rc {
	case 1:
		v := true
		return &v, nil
	case 0:
		v := false
		return &v, nil
	case 2:
		return nil, nil
	default:
		return nil, errors.New(string(result))
	}
}

// ---------------------------------------------------------------------------
// Function Task Compilation
// ---------------------------------------------------------------------------

// CompileFunctionTasks compiles a Function's task expressions for a given input.
func CompileFunctionTasks(function FunctionsFunction, input FunctionsExpressionInputValue) ([]FunctionsCompiledTask, error) {
	out, err := cffi2[FunctionsFunction, FunctionsExpressionInputValue, []FunctionsCompiledTask]("objectiveai_compile_function_tasks", function, input)
	if err != nil {
		return nil, err
	}
	return *out, nil
}

// CompileFunctionOutputLength computes the expected output length for a vector Function.
func CompileFunctionOutputLength(function FunctionsFunction, input FunctionsExpressionInputValue) (*uint32, error) {
	return cffi2[FunctionsFunction, FunctionsExpressionInputValue, uint32]("objectiveai_compile_function_output_length", function, input)
}

// CompileFunctionInputSplit compiles the input_split expression.
func CompileFunctionInputSplit(function FunctionsFunction, input FunctionsExpressionInputValue) ([]FunctionsExpressionInputValue, error) {
	out, err := cffi2[FunctionsFunction, FunctionsExpressionInputValue, []FunctionsExpressionInputValue]("objectiveai_compile_function_input_split", function, input)
	if err != nil {
		return nil, err
	}
	if out == nil {
		return nil, nil
	}
	return *out, nil
}

// CompileFunctionInputMerge compiles the input_merge expression.
func CompileFunctionInputMerge(function FunctionsFunction, input []FunctionsExpressionInputValue) (*FunctionsExpressionInputValue, error) {
	return cffi2[FunctionsFunction, []FunctionsExpressionInputValue, FunctionsExpressionInputValue]("objectiveai_compile_function_input_merge", function, input)
}

// ---------------------------------------------------------------------------
// Vector/Scalar Field Validation
// ---------------------------------------------------------------------------

// CheckVectorFields validates vector function fields.
func CheckVectorFields(fields FunctionsCheckVectorFieldsValidation) error {
	return cffi1Void("objectiveai_check_vector_fields", fields)
}

// CheckScalarFields validates scalar function fields.
func CheckScalarFields(fields FunctionsCheckScalarFieldsValidation) error {
	return cffi1Void("objectiveai_check_scalar_fields", fields)
}

// ---------------------------------------------------------------------------
// Alpha Function Validation
// ---------------------------------------------------------------------------

// AlphaCheckLeafScalarFunction validates a leaf scalar function (depth 0).
func AlphaCheckLeafScalarFunction(function FunctionsAlphaScalarRemoteFunction) error {
	return cffi1Void("objectiveai_alpha_check_leaf_scalar_function", function)
}

// AlphaCheckBranchScalarFunction validates a branch scalar function (depth > 0).
func AlphaCheckBranchScalarFunction(function FunctionsAlphaScalarRemoteFunction, children map[string]FunctionsFullRemoteFunction) error {
	return cffi2Void("objectiveai_alpha_check_branch_scalar_function", function, children)
}

// AlphaCheckLeafVectorFunction validates a leaf vector function (depth 0).
func AlphaCheckLeafVectorFunction(function FunctionsAlphaVectorRemoteFunction) error {
	return cffi1Void("objectiveai_alpha_check_leaf_vector_function", function)
}

// AlphaCheckBranchVectorFunction validates a branch vector function (depth > 0).
func AlphaCheckBranchVectorFunction(function FunctionsAlphaVectorRemoteFunction, children map[string]FunctionsFullRemoteFunction) error {
	return cffi2Void("objectiveai_alpha_check_branch_vector_function", function, children)
}

// ---------------------------------------------------------------------------
// Streaming Chunk Merging
// ---------------------------------------------------------------------------

// AgentCompletionChunkMerged merges two AgentCompletionChunks via push.
func AgentCompletionChunkMerged(a, b AgentCompletionsResponseStreamingAgentCompletionChunk) (*AgentCompletionsResponseStreamingAgentCompletionChunk, error) {
	return cffi2[AgentCompletionsResponseStreamingAgentCompletionChunk, AgentCompletionsResponseStreamingAgentCompletionChunk, AgentCompletionsResponseStreamingAgentCompletionChunk]("objectiveai_agent_completion_chunk_merged", a, b)
}

// VectorCompletionChunkMerged merges two VectorCompletionChunks via push.
func VectorCompletionChunkMerged(a, b VectorCompletionsResponseStreamingVectorCompletionChunk) (*VectorCompletionsResponseStreamingVectorCompletionChunk, error) {
	return cffi2[VectorCompletionsResponseStreamingVectorCompletionChunk, VectorCompletionsResponseStreamingVectorCompletionChunk, VectorCompletionsResponseStreamingVectorCompletionChunk]("objectiveai_vector_completion_chunk_merged", a, b)
}

// FunctionExecutionChunkMerged merges two FunctionExecutionChunks via push.
func FunctionExecutionChunkMerged(a, b FunctionsExecutionsResponseStreamingFunctionExecutionChunk) (*FunctionsExecutionsResponseStreamingFunctionExecutionChunk, error) {
	return cffi2[FunctionsExecutionsResponseStreamingFunctionExecutionChunk, FunctionsExecutionsResponseStreamingFunctionExecutionChunk, FunctionsExecutionsResponseStreamingFunctionExecutionChunk]("objectiveai_function_execution_chunk_merged", a, b)
}

// FunctionProfileComputationChunkMerged merges two FunctionProfileComputationChunks via push.
func FunctionProfileComputationChunkMerged(a, b FunctionsProfilesComputationsResponseStreamingFunctionProfileComputationChunk) (*FunctionsProfilesComputationsResponseStreamingFunctionProfileComputationChunk, error) {
	return cffi2[FunctionsProfilesComputationsResponseStreamingFunctionProfileComputationChunk, FunctionsProfilesComputationsResponseStreamingFunctionProfileComputationChunk, FunctionsProfilesComputationsResponseStreamingFunctionProfileComputationChunk]("objectiveai_function_profile_computation_chunk_merged", a, b)
}

// ---------------------------------------------------------------------------
// Streaming Chunk Normalization
// ---------------------------------------------------------------------------

// AgentCompletionChunkNormalized normalizes an AgentCompletionChunk.
func AgentCompletionChunkNormalized(chunk AgentCompletionsResponseStreamingAgentCompletionChunk) (*AgentCompletionsResponseStreamingAgentCompletionChunk, error) {
	return cffi1[AgentCompletionsResponseStreamingAgentCompletionChunk, AgentCompletionsResponseStreamingAgentCompletionChunk]("objectiveai_agent_completion_chunk_normalized", chunk)
}

// VectorCompletionChunkNormalized normalizes a VectorCompletionChunk.
func VectorCompletionChunkNormalized(chunk VectorCompletionsResponseStreamingVectorCompletionChunk) (*VectorCompletionsResponseStreamingVectorCompletionChunk, error) {
	return cffi1[VectorCompletionsResponseStreamingVectorCompletionChunk, VectorCompletionsResponseStreamingVectorCompletionChunk]("objectiveai_vector_completion_chunk_normalized", chunk)
}

// FunctionExecutionChunkNormalized normalizes a FunctionExecutionChunk.
func FunctionExecutionChunkNormalized(chunk FunctionsExecutionsResponseStreamingFunctionExecutionChunk) (*FunctionsExecutionsResponseStreamingFunctionExecutionChunk, error) {
	return cffi1[FunctionsExecutionsResponseStreamingFunctionExecutionChunk, FunctionsExecutionsResponseStreamingFunctionExecutionChunk]("objectiveai_function_execution_chunk_normalized", chunk)
}

// FunctionProfileComputationChunkNormalized normalizes a FunctionProfileComputationChunk.
func FunctionProfileComputationChunkNormalized(chunk FunctionsProfilesComputationsResponseStreamingFunctionProfileComputationChunk) (*FunctionsProfilesComputationsResponseStreamingFunctionProfileComputationChunk, error) {
	return cffi1[FunctionsProfilesComputationsResponseStreamingFunctionProfileComputationChunk, FunctionsProfilesComputationsResponseStreamingFunctionProfileComputationChunk]("objectiveai_function_profile_computation_chunk_normalized", chunk)
}

// ---------------------------------------------------------------------------
// Streaming Chunk to Unary Conversion
// ---------------------------------------------------------------------------

// AgentCompletionChunkToUnary converts an accumulated chunk to a unary AgentCompletion.
func AgentCompletionChunkToUnary(chunk AgentCompletionsResponseStreamingAgentCompletionChunk) (*AgentCompletionsResponseUnaryAgentCompletion, error) {
	return cffi1[AgentCompletionsResponseStreamingAgentCompletionChunk, AgentCompletionsResponseUnaryAgentCompletion]("objectiveai_agent_completion_chunk_to_unary", chunk)
}

// VectorCompletionChunkToUnary converts an accumulated chunk to a unary VectorCompletion.
func VectorCompletionChunkToUnary(chunk VectorCompletionsResponseStreamingVectorCompletionChunk) (*VectorCompletionsResponseUnaryVectorCompletion, error) {
	return cffi1[VectorCompletionsResponseStreamingVectorCompletionChunk, VectorCompletionsResponseUnaryVectorCompletion]("objectiveai_vector_completion_chunk_to_unary", chunk)
}

// FunctionExecutionChunkToUnary converts an accumulated chunk to a unary FunctionExecution.
func FunctionExecutionChunkToUnary(chunk FunctionsExecutionsResponseStreamingFunctionExecutionChunk) (*FunctionsExecutionsResponseUnaryFunctionExecution, error) {
	return cffi1[FunctionsExecutionsResponseStreamingFunctionExecutionChunk, FunctionsExecutionsResponseUnaryFunctionExecution]("objectiveai_function_execution_chunk_to_unary", chunk)
}

// FunctionProfileComputationChunkToUnary converts an accumulated chunk to a unary FunctionProfileComputation.
func FunctionProfileComputationChunkToUnary(chunk FunctionsProfilesComputationsResponseStreamingFunctionProfileComputationChunk) (*FunctionsProfilesComputationsResponseUnaryFunctionProfileComputation, error) {
	return cffi1[FunctionsProfilesComputationsResponseStreamingFunctionProfileComputationChunk, FunctionsProfilesComputationsResponseUnaryFunctionProfileComputation]("objectiveai_function_profile_computation_chunk_to_unary", chunk)
}

// ---------------------------------------------------------------------------
// Normalize Unary Responses (for tests)
// ---------------------------------------------------------------------------

// NormalizeAgentCompletionForTests normalizes an AgentCompletion by round-tripping through serde.
func NormalizeAgentCompletionForTests(v AgentCompletionsResponseUnaryAgentCompletion) (*AgentCompletionsResponseUnaryAgentCompletion, error) {
	return cffi1[AgentCompletionsResponseUnaryAgentCompletion, AgentCompletionsResponseUnaryAgentCompletion]("objectiveai_normalize_agent_completion_for_tests", v)
}

// NormalizeVectorCompletionForTests normalizes a VectorCompletion by round-tripping through serde.
func NormalizeVectorCompletionForTests(v VectorCompletionsResponseUnaryVectorCompletion) (*VectorCompletionsResponseUnaryVectorCompletion, error) {
	return cffi1[VectorCompletionsResponseUnaryVectorCompletion, VectorCompletionsResponseUnaryVectorCompletion]("objectiveai_normalize_vector_completion_for_tests", v)
}

// NormalizeFunctionExecutionForTests normalizes a FunctionExecution by round-tripping through serde.
func NormalizeFunctionExecutionForTests(v FunctionsExecutionsResponseUnaryFunctionExecution) (*FunctionsExecutionsResponseUnaryFunctionExecution, error) {
	return cffi1[FunctionsExecutionsResponseUnaryFunctionExecution, FunctionsExecutionsResponseUnaryFunctionExecution]("objectiveai_normalize_function_execution_for_tests", v)
}

// NormalizeFunctionProfileComputationForTests normalizes a FunctionProfileComputation by round-tripping through serde.
func NormalizeFunctionProfileComputationForTests(v FunctionsProfilesComputationsResponseUnaryFunctionProfileComputation) (*FunctionsProfilesComputationsResponseUnaryFunctionProfileComputation, error) {
	return cffi1[FunctionsProfilesComputationsResponseUnaryFunctionProfileComputation, FunctionsProfilesComputationsResponseUnaryFunctionProfileComputation]("objectiveai_normalize_function_profile_computation_for_tests", v)
}

// ---------------------------------------------------------------------------
// Generate Arbitrary Chunks
// ---------------------------------------------------------------------------

// GenerateAgentCompletionChunk generates a random AgentCompletionChunk from a seed.
func GenerateAgentCompletionChunk(hasSeed bool, seed int64) (*AgentCompletionsResponseStreamingAgentCompletionChunk, error) {
	return cffiGenerate[AgentCompletionsResponseStreamingAgentCompletionChunk]("objectiveai_generate_agent_completion_chunk", hasSeed, seed)
}

// GenerateVectorCompletionChunk generates a random VectorCompletionChunk from a seed.
func GenerateVectorCompletionChunk(hasSeed bool, seed int64) (*VectorCompletionsResponseStreamingVectorCompletionChunk, error) {
	return cffiGenerate[VectorCompletionsResponseStreamingVectorCompletionChunk]("objectiveai_generate_vector_completion_chunk", hasSeed, seed)
}

// GenerateFunctionExecutionChunk generates a random FunctionExecutionChunk from a seed.
func GenerateFunctionExecutionChunk(hasSeed bool, seed int64) (*FunctionsExecutionsResponseStreamingFunctionExecutionChunk, error) {
	return cffiGenerate[FunctionsExecutionsResponseStreamingFunctionExecutionChunk]("objectiveai_generate_function_execution_chunk", hasSeed, seed)
}

// GenerateFunctionProfileComputationChunk generates a random FunctionProfileComputationChunk from a seed.
func GenerateFunctionProfileComputationChunk(hasSeed bool, seed int64) (*FunctionsProfilesComputationsResponseStreamingFunctionProfileComputationChunk, error) {
	return cffiGenerate[FunctionsProfilesComputationsResponseStreamingFunctionProfileComputationChunk]("objectiveai_generate_function_profile_computation_chunk", hasSeed, seed)
}
