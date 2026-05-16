//! Shared invention-test helpers, request builder, and macros used by
//! the single inventions integration binary
//! (`tests/functions_inventions.rs` plus its sibling-file modules
//! under `tests/functions_inventions/`).
//!
//! Every inventions test — scalar, vector, schema-provided variants,
//! validation/error checks, and the recursive (depth-1+) suite —
//! compiles into that one binary so the suite shares **exactly one**
//! spawned api server subprocess (held by the
//! `LazyLock<ServerHandle>` in `tests/common/server.rs`). Cargo
//! treats every `tests/*.rs` file as its own test binary, so the
//! tests are organised into `tests/functions_inventions/{scalar,
//! vector, validation, recursive}.rs` modules pulled in via
//! `mod` declarations from `tests/functions_inventions.rs` — sibling
//! files inside a subdirectory aren't auto-discovered as separate
//! binaries by cargo, so we keep the four-category source layout
//! while paying the cost of just one binary and one server
//! subprocess.
//!
//! Snapshot file paths in the `invention_test_10x!` /
//! `recursive_test_3x!` macros use
//! `concat!(env!("CARGO_MANIFEST_DIR"), "/assets/...")` for both
//! `concat!` (the `path:` argument) and `include_str!` (the embedded
//! snapshot bytes), so the macros work whether they're invoked from a
//! `tests/*.rs` root file or a `tests/*/`-subdirectory module file.

use futures::StreamExt;

use objectiveai_sdk::functions::expression::{
    AnyOfInputSchema, ArrayInputSchema, AudioInputSchema, BooleanInputSchema,
    FileInputSchema, ImageInputSchema, IntegerInputSchema, InputSchema, NumberInputSchema,
    ObjectInputSchema, StringInputSchema, VideoInputSchema,
};
use objectiveai_sdk::functions::alpha_vector::expression::VectorFunctionInputSchema;
use objectiveai_sdk::functions::inventions::request::FunctionInventionCreateParams;
use objectiveai_sdk::functions::inventions::recursive::request::FunctionInventionRecursiveCreateParams;
use objectiveai_sdk::functions::inventions::recursive::response::streaming::FunctionInventionRecursiveChunk;
use objectiveai_sdk::functions::inventions::recursive::response::unary::FunctionInventionRecursive;
use objectiveai_sdk::functions::inventions::response::streaming::FunctionInventionChunk;
use objectiveai_sdk::functions::inventions::response::unary::FunctionInvention;
use objectiveai_sdk::functions::inventions::state::{Params, ParamsState};

pub fn make_request(state: ParamsState, seed: i64) -> FunctionInventionCreateParams {
    FunctionInventionCreateParams {
        remote: None,
        overwrite: None,
        state: objectiveai_sdk::functions::inventions::ParamsStateOrRemoteCommitOptional::Inline(state),
        provider: None,
        agent: objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
            objectiveai_sdk::agent::InlineAgentBaseWithFallbacks {
                inner: objectiveai_sdk::agent::InlineAgentBase::Mock(objectiveai_sdk::agent::mock::AgentBase {
                    mode: Some(objectiveai_sdk::agent::mock::Mode::Invention),
                    ..Default::default()
                }),
                fallbacks: None,
            },
        ),
        prompt: objectiveai_sdk::functions::inventions::prompts::InlinePromptOrRemoteCommitOptional::Remote(
            objectiveai_sdk::RemotePathCommitOptional::Mock { name: "default".to_string() },
        ),
        seed: Some(seed),
        stream: Some(true),
        max_step_retries: Some(1),
        continuation: None,
    }
}

pub fn params(name: &str, depth: u64, min_b: u64, max_b: u64, min_l: u64, max_l: u64) -> Params {
    Params {
        depth,
        min_branch_width: min_b,
        max_branch_width: max_b,
        min_leaf_width: min_l,
        max_leaf_width: max_l,
        name: name.to_string(),
        spec: "Test function spec for mock invention.".to_string(),
    }
}

fn check_created(expected: &std::cell::Cell<Option<u64>>, i: usize, created: u64) {
    match expected.get() {
        None => expected.set(Some(created)),
        Some(exp) => assert_eq!(created, exp, "chunk {i} has created {created}, expected {exp}"),
    }
}

pub async fn post_streaming(
    params: FunctionInventionCreateParams,
) -> Result<impl futures::Stream<Item = FunctionInventionChunk> + Unpin, String> {
    let http = super::server::client();
    let stream = http
        .send_streaming::<FunctionInventionChunk, _, _>(
            reqwest::Method::POST,
            "/functions/inventions",
            Some(params),
        )
        .await
        .map_err(|e| format!("{e:?}"))?;
    Ok(Box::pin(stream.map(|item| match item {
        Ok(chunk) => chunk,
        Err(e) => panic!("chunk deserialize / stream error: {e:?}"),
    })))
}

pub async fn post_expect_err(params: FunctionInventionCreateParams) -> String {
    let http = super::server::client();
    let result = http
        .send_streaming::<FunctionInventionChunk, _, _>(
            reqwest::Method::POST,
            "/functions/inventions",
            Some(params),
        )
        .await;
    let mut stream = match result {
        Ok(s) => Box::pin(s),
        Err(e) => return format!("{e:?}"),
    };
    while let Some(item) = stream.next().await {
        match item {
            Ok(chunk) => {
                if let Some(err) = &chunk.error {
                    return format!("{err:?}");
                }
            }
            Err(e) => return format!("{e:?}"),
        }
    }
    panic!("expected an error, but stream ended without one");
}

pub async fn run_invention(params: FunctionInventionCreateParams) -> FunctionInvention {
    let stream = post_streaming(params)
        .await
        .expect("invention should not error before streaming");
    let expected_created = std::cell::Cell::new(None);
    let mut errors: Vec<objectiveai_sdk::error::ResponseError> = Vec::new();
    let agg = super::stream_harness::consume_stream_acc(
        stream,
        |agg, c| agg.push(c),
        |chunk, errors_acc: &mut Vec<objectiveai_sdk::error::ResponseError>| {
            if let Some(e) = &chunk.error {
                errors_acc.push(e.clone());
            }
            for completion in &chunk.completions {
                if let Some(e) = &completion.inner.error {
                    errors_acc.push(e.clone());
                }
            }
        },
        |i, chunk| {
            check_created(&expected_created, i, chunk.created);
            assert!(chunk.completions.len() <= 1, "chunk {i} has {} completions, expected at most 1", chunk.completions.len());
            assert!(chunk.usage.is_none(), "chunk {i} (non-final) has usage, expected None");
            assert!(chunk.function.is_none(), "chunk {i} (non-final) has function, expected None");
        },
        |i, chunk| {
            check_created(&expected_created, i, chunk.created);
            assert!(chunk.completions.len() <= 1, "chunk {i} has {} completions, expected at most 1", chunk.completions.len());
            assert!(chunk.usage.is_none(), "chunk {i} (non-final) has usage, expected None");
            assert!(chunk.function.is_none(), "chunk {i} (non-final) has function, expected None");
        },
        |i, chunk, errs| {
            check_created(&expected_created, i, chunk.created);
            assert!(chunk.usage.is_some(), "final chunk {i} has no usage, expected Some");
            assert!(chunk.function.is_some(), "final chunk {i} has no function, expected Some. errors: {}", serde_json::to_string(errs).unwrap());
            assert!(chunk.state.is_some(), "final chunk {i} has no state, expected Some");
        },
        std::mem::take(&mut errors),
    ).await;
    FunctionInvention::from(agg)
}

pub fn normalize(mut fi: FunctionInvention) -> FunctionInvention {
    fi.normalize_for_tests();
    fi
}

pub fn assert_snapshot(json: &str, path: &str, expected: &str) {
    super::stream_harness::assert_snapshot(
        json, path, expected,
        "UPDATE_FUNCTIONS_INVENTIONS_CLIENT_TESTS_SNAPSHOTS",
    );
}

// ---------------------------------------------------------------------------
// Schema helpers
// ---------------------------------------------------------------------------

pub fn obj(props: Vec<(&str, InputSchema)>, required: Vec<&str>) -> ObjectInputSchema {
    ObjectInputSchema {
        r#type: Default::default(),
        description: None,
        properties: props.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
        required: Some(required.into_iter().map(String::from).collect()),
    }
}

pub fn arr(items: InputSchema, min: u64, max: u64) -> InputSchema {
    InputSchema::Array(ArrayInputSchema {
        r#type: Default::default(),
        description: None,
        items: Box::new(items),
        min_items: Some(min),
        max_items: Some(max),
    })
}

pub fn any_of(schemas: Vec<InputSchema>) -> InputSchema {
    InputSchema::AnyOf(AnyOfInputSchema { any_of: schemas })
}

pub fn string() -> InputSchema { InputSchema::String(StringInputSchema { r#type: Default::default(), description: None, r#enum: None }) }
pub fn integer() -> InputSchema { InputSchema::Integer(IntegerInputSchema { r#type: Default::default(), description: None, minimum: None, maximum: None }) }
pub fn number() -> InputSchema { InputSchema::Number(NumberInputSchema { r#type: Default::default(), description: None, minimum: None, maximum: None }) }
pub fn boolean() -> InputSchema { InputSchema::Boolean(BooleanInputSchema { r#type: Default::default(), description: None }) }
pub fn image() -> InputSchema { InputSchema::Image(ImageInputSchema { r#type: Default::default(), description: None }) }
pub fn audio() -> InputSchema { InputSchema::Audio(AudioInputSchema { r#type: Default::default(), description: None }) }
pub fn video() -> InputSchema { InputSchema::Video(VideoInputSchema { r#type: Default::default(), description: None }) }
pub fn file() -> InputSchema { InputSchema::File(FileInputSchema { r#type: Default::default(), description: None }) }
pub fn nested_obj(props: Vec<(&str, InputSchema)>, required: Vec<&str>) -> InputSchema {
    InputSchema::Object(obj(props, required))
}

pub fn scalar_schema_anyof_chaos() -> ObjectInputSchema {
    obj(
        vec![
            ("payload", any_of(vec![
                string(),
                image(),
                nested_obj(vec![
                    ("caption", string()),
                    ("score", number()),
                ], vec!["caption"]),
            ])),
            ("tags", arr(string(), 1, 10)),
            ("priority", integer()),
        ],
        vec!["payload", "tags", "priority"],
    )
}

pub fn scalar_schema_deep_media() -> ObjectInputSchema {
    obj(
        vec![
            ("submission", nested_obj(vec![
                ("content", nested_obj(vec![
                    ("body", nested_obj(vec![
                        ("text", string()),
                        ("attachment", any_of(vec![image(), audio(), video(), file()])),
                    ], vec!["text", "attachment"])),
                    ("metadata", nested_obj(vec![
                        ("author", string()),
                        ("timestamp", integer()),
                    ], vec!["author"])),
                ], vec!["body"])),
            ], vec!["content"])),
            ("verified", boolean()),
        ],
        vec!["submission", "verified"],
    )
}

pub fn scalar_schema_array_madness() -> ObjectInputSchema {
    obj(
        vec![
            ("entries", arr(
                any_of(vec![
                    string(),
                    nested_obj(vec![
                        ("label", string()),
                        ("values", arr(any_of(vec![number(), integer(), boolean()]), 1, 5)),
                        ("photo", image()),
                    ], vec!["label", "values", "photo"]),
                ]),
                2, 20,
            )),
            ("query", string()),
        ],
        vec!["entries", "query"],
    )
}

pub fn scalar_schema_kitchen_sink() -> ObjectInputSchema {
    obj(
        vec![
            ("name", string()),
            ("age", integer()),
            ("score", number()),
            ("active", boolean()),
            ("avatar", image()),
            ("voicemail", audio()),
            ("demo", video()),
            ("resume", file()),
            ("aliases", arr(any_of(vec![string(), integer()]), 1, 8)),
            ("extra", any_of(vec![
                string(),
                arr(nested_obj(vec![
                    ("key", string()),
                    ("val", any_of(vec![number(), boolean(), image()])),
                ], vec!["key", "val"]), 1, 3),
            ])),
        ],
        vec!["name", "age", "score", "active", "avatar", "voicemail", "demo", "resume", "aliases", "extra"],
    )
}

pub fn vector_schema_multimedia_ranking() -> VectorFunctionInputSchema {
    VectorFunctionInputSchema {
        context: Some(obj(
            vec![
                ("criteria", arr(string(), 1, 5)),
                ("reference_image", image()),
            ],
            vec!["criteria", "reference_image"],
        )),
        items: arr(
            any_of(vec![
                image(),
                nested_obj(vec![
                    ("visual", image()),
                    ("caption", string()),
                    ("metadata", any_of(vec![
                        string(),
                        nested_obj(vec![
                            ("source", string()),
                            ("confidence", number()),
                        ], vec!["source", "confidence"]),
                    ])),
                ], vec!["visual", "caption"]),
            ]),
            2, 6,
        ),
    }
}

pub fn vector_schema_nested_chaos() -> VectorFunctionInputSchema {
    VectorFunctionInputSchema {
        context: Some(obj(
            vec![
                ("prompt", string()),
                ("settings", nested_obj(vec![
                    ("temperature", number()),
                    ("mode", any_of(vec![string(), integer()])),
                ], vec!["temperature"])),
            ],
            vec!["prompt"],
        )),
        items: arr(
            nested_obj(vec![
                ("content", any_of(vec![
                    string(),
                    nested_obj(vec![
                        ("parts", arr(any_of(vec![string(), image(), audio()]), 1, 2)),
                        ("format", string()),
                    ], vec!["parts", "format"]),
                ])),
                ("scores", arr(number(), 1, 2)),
            ], vec!["content", "scores"]),
            2, 4,
        ),
    }
}

pub fn vector_schema_rich_context() -> VectorFunctionInputSchema {
    VectorFunctionInputSchema {
        context: Some(obj(
            vec![
                ("rubric", nested_obj(vec![
                    ("dimensions", arr(
                        nested_obj(vec![
                            ("name", string()),
                            ("weight", number()),
                            ("examples", arr(any_of(vec![string(), image()]), 1, 2)),
                        ], vec!["name", "weight"]),
                        1, 2,
                    )),
                    ("passing_threshold", number()),
                ], vec!["dimensions", "passing_threshold"])),
                ("evaluator", any_of(vec![
                    string(),
                    nested_obj(vec![
                        ("id", integer()),
                        ("credentials", arr(string(), 0, 2)),
                    ], vec!["id"]),
                ])),
            ],
            vec!["rubric"],
        )),
        items: arr(string(), 2, 4),
    }
}

pub fn vector_schema_no_context_deep_items() -> VectorFunctionInputSchema {
    VectorFunctionInputSchema {
        context: None,
        items: arr(
            nested_obj(vec![
                ("candidate", any_of(vec![
                    file(),
                    nested_obj(vec![
                        ("document", file()),
                        ("preview", any_of(vec![image(), video()])),
                    ], vec!["document"]),
                ])),
                ("relevance_hint", any_of(vec![string(), number()])),
            ], vec!["candidate"]),
            2, 3,
        ),
    }
}

// ---------------------------------------------------------------------------
// Test macros
// ---------------------------------------------------------------------------

#[macro_export]
macro_rules! invention_test_10x {
    (
        $test_name:ident,
        $variant:ident, $state_ty:ident,
        $name:expr, $depth:expr,
        $min_b:expr, $max_b:expr, $min_l:expr, $max_l:expr,
        $base_seed:expr,
        $base:expr
    ) => {
        mod $test_name {
            use $crate::common::inventions::*;
            use objectiveai_sdk::functions::inventions::state::{
                AlphaScalarLeafState, AlphaScalarBranchState, AlphaVectorLeafState,
                AlphaVectorBranchState, ParamsState,
            };

            fn make_state(seed_offset: i64) -> (ParamsState, i64) {
                (
                    ParamsState::$variant($state_ty {
                        params: params($name, $depth, $min_b, $max_b, $min_l, $max_l),
                        essay: None,
                        input_schema: None,
                        essay_tasks: None,
                        tasks: None,
                        tasks_length: None,
                        description: None,
                        readme: None,
                        checker_seed: None,
                    }),
                    ($base_seed as i64) + seed_offset,
                )
            }

            async fn run_snapshot(offset: i64, path: &str, expected: &str) {
                let (state, seed) = make_state(offset);
                let request = make_request(state, seed);
                let result = normalize(run_invention(request).await);
                assert!(result.function.is_some(), "seed {seed}: function should be built. error: {:?}", result.error);
                let json = serde_json::to_string_pretty(&result).unwrap();
                assert_snapshot(&json, path, expected);
            }

            #[tokio::test] async fn seed_0() {
                run_snapshot(0,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_0.json"),
                    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_0.json"))).await;
            }
            #[tokio::test] async fn seed_1() {
                run_snapshot(1,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_1.json"),
                    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_1.json"))).await;
            }
            #[tokio::test] async fn seed_2() {
                run_snapshot(2,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_2.json"),
                    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_2.json"))).await;
            }
            #[tokio::test] async fn seed_3() {
                run_snapshot(3,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_3.json"),
                    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_3.json"))).await;
            }
            #[tokio::test] async fn seed_4() {
                run_snapshot(4,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_4.json"),
                    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_4.json"))).await;
            }
            #[tokio::test] async fn seed_5() {
                run_snapshot(5,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_5.json"),
                    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_5.json"))).await;
            }
            #[tokio::test] async fn seed_6() {
                run_snapshot(6,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_6.json"),
                    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_6.json"))).await;
            }
            #[tokio::test] async fn seed_7() {
                run_snapshot(7,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_7.json"),
                    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_7.json"))).await;
            }
            #[tokio::test] async fn seed_8() {
                run_snapshot(8,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_8.json"),
                    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_8.json"))).await;
            }
            #[tokio::test] async fn seed_9() {
                run_snapshot(9,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_9.json"),
                    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_9.json"))).await;
            }
        }
    };
}

#[macro_export]
macro_rules! invention_test_10x_schema {
    (
        $test_name:ident,
        $variant:ident, $state_ty:ident,
        $name:expr, $depth:expr,
        $min_b:expr, $max_b:expr, $min_l:expr, $max_l:expr,
        $base_seed:expr,
        $base:expr,
        $schema:expr
    ) => {
        mod $test_name {
            use $crate::common::inventions::*;
            use objectiveai_sdk::functions::inventions::state::{
                AlphaScalarLeafState, AlphaScalarBranchState, AlphaVectorLeafState,
                AlphaVectorBranchState, ParamsState,
            };

            fn make_state(seed_offset: i64) -> (ParamsState, i64) {
                (
                    ParamsState::$variant($state_ty {
                        params: params($name, $depth, $min_b, $max_b, $min_l, $max_l),
                        essay: None,
                        input_schema: Some($schema),
                        essay_tasks: None,
                        tasks: None,
                        tasks_length: None,
                        description: None,
                        readme: None,
                        checker_seed: None,
                    }),
                    ($base_seed as i64) + seed_offset,
                )
            }

            async fn run_snapshot(offset: i64, path: &str, expected: &str) {
                let (state, seed) = make_state(offset);
                let request = make_request(state, seed);
                let result = normalize(run_invention(request).await);
                assert!(result.function.is_some(), "seed {seed}: function should be built. error: {:?}", result.error);
                let json = serde_json::to_string_pretty(&result).unwrap();
                assert_snapshot(&json, path, expected);
            }

            #[tokio::test] async fn seed_0() {
                run_snapshot(0,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_0.json"),
                    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_0.json"))).await;
            }
            #[tokio::test] async fn seed_1() {
                run_snapshot(1,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_1.json"),
                    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_1.json"))).await;
            }
            #[tokio::test] async fn seed_2() {
                run_snapshot(2,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_2.json"),
                    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_2.json"))).await;
            }
            #[tokio::test] async fn seed_3() {
                run_snapshot(3,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_3.json"),
                    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_3.json"))).await;
            }
            #[tokio::test] async fn seed_4() {
                run_snapshot(4,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_4.json"),
                    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_4.json"))).await;
            }
            #[tokio::test] async fn seed_5() {
                run_snapshot(5,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_5.json"),
                    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_5.json"))).await;
            }
            #[tokio::test] async fn seed_6() {
                run_snapshot(6,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_6.json"),
                    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_6.json"))).await;
            }
            #[tokio::test] async fn seed_7() {
                run_snapshot(7,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_7.json"),
                    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_7.json"))).await;
            }
            #[tokio::test] async fn seed_8() {
                run_snapshot(8,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_8.json"),
                    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_8.json"))).await;
            }
            #[tokio::test] async fn seed_9() {
                run_snapshot(9,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_9.json"),
                    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_9.json"))).await;
            }
        }
    };
}
// ---------------------------------------------------------------------------
// Recursive invention helpers
// ---------------------------------------------------------------------------

pub fn make_recursive_request(
    state: ParamsState,
    seed: i64,
) -> FunctionInventionRecursiveCreateParams {
    FunctionInventionRecursiveCreateParams {
        remote: objectiveai_sdk::Remote::Mock,
        overwrite: None,
        state: objectiveai_sdk::functions::inventions::ParamsStateOrRemoteCommitOptional::Inline(state),
        provider: None,
        agent: objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
            objectiveai_sdk::agent::InlineAgentBaseWithFallbacks {
                inner: objectiveai_sdk::agent::InlineAgentBase::Mock(objectiveai_sdk::agent::mock::AgentBase {
                    mode: Some(objectiveai_sdk::agent::mock::Mode::Invention),
                    ..Default::default()
                }),
                fallbacks: None,
            },
        ),
        prompt: objectiveai_sdk::functions::inventions::prompts::InlinePromptOrRemoteCommitOptional::Remote(
            objectiveai_sdk::RemotePathCommitOptional::Mock { name: "default".to_string() },
        ),
        seed: Some(seed),
        stream: Some(true),
        max_step_retries: Some(1),
        continuation: None,
    }
}

pub async fn post_recursive_streaming(
    params: FunctionInventionRecursiveCreateParams,
) -> Result<impl futures::Stream<Item = FunctionInventionRecursiveChunk> + Unpin, String> {
    let http = super::server::client();
    let stream = http
        .send_streaming::<FunctionInventionRecursiveChunk, _, _>(
            reqwest::Method::POST,
            "/functions/inventions/recursive",
            Some(params),
        )
        .await
        .map_err(|e| format!("{e:?}"))?;
    Ok(Box::pin(stream.map(|item| match item {
        Ok(chunk) => chunk,
        Err(e) => panic!("recursive chunk deserialize / stream error: {e:?}"),
    })))
}

pub async fn post_recursive_expect_err(
    params: FunctionInventionRecursiveCreateParams,
) -> String {
    let http = super::server::client();
    let result = http
        .send_streaming::<FunctionInventionRecursiveChunk, _, _>(
            reqwest::Method::POST,
            "/functions/inventions/recursive",
            Some(params),
        )
        .await;
    let mut stream = match result {
        Ok(s) => Box::pin(s),
        Err(e) => return format!("{e:?}"),
    };
    while let Some(item) = stream.next().await {
        match item {
            Ok(chunk) => {
                if chunk.inventions_errors == Some(true) {
                    for inv in &chunk.inventions {
                        if let Some(err) = &inv.inner.error {
                            return format!("{err:?}");
                        }
                    }
                    return "inventions_errors=true but no error payload".to_string();
                }
            }
            Err(e) => return format!("{e:?}"),
        }
    }
    panic!("expected an error, but stream ended without one");
}

pub async fn run_recursive_invention(
    params: FunctionInventionRecursiveCreateParams,
) -> FunctionInventionRecursive {
    let stream = post_recursive_streaming(params)
        .await
        .expect("recursive invention should not error before streaming");
    let expected_created = std::cell::Cell::new(None);
    let agg = super::stream_harness::consume_stream(
        stream,
        |agg, c| agg.push(c),
        |i, chunk| {
            check_created(&expected_created, i, chunk.created);
            assert_eq!(chunk.inventions.len(), 1, "chunk {i} (non-final) has {} invention chunks, expected exactly 1", chunk.inventions.len());
            assert!(chunk.usage.is_none(), "chunk {i} (non-final) has usage, expected None");
        },
        |i, chunk| {
            check_created(&expected_created, i, chunk.created);
            assert_eq!(chunk.inventions.len(), 1, "chunk {i} (non-final) has {} invention chunks, expected exactly 1", chunk.inventions.len());
            assert!(chunk.usage.is_none(), "chunk {i} (non-final) has usage, expected None");
        },
        |i, chunk| {
            check_created(&expected_created, i, chunk.created);
            assert_eq!(chunk.inventions.len(), 0, "final chunk {i} has {} invention chunks, expected 0", chunk.inventions.len());
            assert!(chunk.usage.is_some(), "final chunk {i} has no usage, expected Some");
        },
    ).await;
    FunctionInventionRecursive::from(agg)
}

pub fn normalize_recursive(mut fi: FunctionInventionRecursive) -> FunctionInventionRecursive {
    fi.normalize_for_tests();
    fi
}

pub fn assert_recursive_snapshot(json: &str, path: &str, expected: &str) {
    super::stream_harness::assert_snapshot(
        json, path, expected,
        "UPDATE_FUNCTIONS_INVENTIONS_RECURSIVE_CLIENT_TESTS_SNAPSHOTS",
    );
}

// ---------------------------------------------------------------------------
// Recursive-test schema and task helpers (used by the
// initial-state-validation tests).
// ---------------------------------------------------------------------------

pub fn valid_scalar_schema() -> objectiveai_sdk::functions::alpha_scalar::expression::ScalarFunctionInputSchema {
    let mut properties = indexmap::IndexMap::new();
    properties.insert(
        "sentiment".to_string(),
        InputSchema::String(StringInputSchema {
            r#type: Default::default(),
            description: None,
            r#enum: Some(vec!["positive".to_string(), "negative".to_string()]),
        }),
    );
    ObjectInputSchema {
        r#type: Default::default(),
        description: None,
        properties,
        required: Some(vec!["sentiment".to_string()]),
    }
}

pub fn invalid_scalar_schema() -> objectiveai_sdk::functions::alpha_scalar::expression::ScalarFunctionInputSchema {
    let mut properties = indexmap::IndexMap::new();
    properties.insert(
        "mood".to_string(),
        InputSchema::String(StringInputSchema {
            r#type: Default::default(),
            description: None,
            r#enum: Some(vec!["sad".to_string()]),
        }),
    );
    ObjectInputSchema {
        r#type: Default::default(),
        description: None,
        properties,
        required: Some(vec!["mood".to_string()]),
    }
}

pub fn valid_vector_schema() -> objectiveai_sdk::functions::alpha_vector::expression::VectorFunctionInputSchema {
    objectiveai_sdk::functions::alpha_vector::expression::VectorFunctionInputSchema {
        context: None,
        items: InputSchema::String(StringInputSchema {
            r#type: Default::default(),
            description: None,
            r#enum: Some(vec!["apple".to_string(), "banana".to_string()]),
        }),
    }
}

pub fn valid_scalar_leaf_task() -> objectiveai_sdk::functions::alpha_scalar::LeafTaskExpression {
    objectiveai_sdk::functions::alpha_scalar::LeafTaskExpression::VectorCompletion(
        objectiveai_sdk::functions::alpha_scalar::VectorCompletionTaskExpression {
            skip: None,
            messages: objectiveai_sdk::functions::expression::Expression::Starlark(
                "[{\"role\": \"user\", \"content\": [{\"type\": \"text\", \"text\": str(input)}]}]".to_string(),
            ),
            responses: vec![
                objectiveai_sdk::agent::completions::message::RichContent::Parts(vec![
                    objectiveai_sdk::agent::completions::message::RichContentPart::Text { text: "yes".to_string() },
                ]),
                objectiveai_sdk::agent::completions::message::RichContent::Parts(vec![
                    objectiveai_sdk::agent::completions::message::RichContentPart::Text { text: "no".to_string() },
                ]),
            ],
        },
    )
}

pub fn invalid_scalar_leaf_task() -> objectiveai_sdk::functions::alpha_scalar::LeafTaskExpression {
    objectiveai_sdk::functions::alpha_scalar::LeafTaskExpression::VectorCompletion(
        objectiveai_sdk::functions::alpha_scalar::VectorCompletionTaskExpression {
            skip: None,
            messages: objectiveai_sdk::functions::expression::Expression::Starlark(
                "[{\"role\": \"user\", \"content\": [{\"type\": \"text\", \"text\": \"hardcoded\"}]}]".to_string(),
            ),
            responses: vec![
                objectiveai_sdk::agent::completions::message::RichContent::Parts(vec![
                    objectiveai_sdk::agent::completions::message::RichContentPart::Text { text: "yes".to_string() },
                ]),
                objectiveai_sdk::agent::completions::message::RichContent::Parts(vec![
                    objectiveai_sdk::agent::completions::message::RichContentPart::Text { text: "no".to_string() },
                ]),
            ],
        },
    )
}

pub fn valid_vector_leaf_task() -> objectiveai_sdk::functions::alpha_vector::LeafTaskExpression {
    objectiveai_sdk::functions::alpha_vector::LeafTaskExpression::VectorCompletion(
        objectiveai_sdk::functions::alpha_vector::VectorCompletionTaskExpression {
            skip: None,
            messages: objectiveai_sdk::functions::expression::Expression::Starlark(
                "[{\"role\": \"user\", \"content\": [{\"type\": \"text\", \"text\": \"rank these\"}]}]".to_string(),
            ),
            responses: objectiveai_sdk::functions::expression::Expression::Starlark(
                "[[{\"type\": \"text\", \"text\": str(item)}] for item in input['items']]".to_string(),
            ),
        },
    )
}

// ---------------------------------------------------------------------------
// Recursive snapshot test macros — 3 seeds per test (recursive tests
// are heavier than the per-shape inventions).
// ---------------------------------------------------------------------------

#[macro_export]
macro_rules! recursive_test_3x {
    (
        $test_name:ident,
        $variant:ident, $state_ty:ident,
        $name:expr, $depth:expr,
        $min_b:expr, $max_b:expr, $min_l:expr, $max_l:expr,
        $base_seed:expr,
        $base:expr
    ) => {
        mod $test_name {
            use $crate::common::inventions::*;
            use objectiveai_sdk::functions::inventions::state::{
                AlphaScalarLeafState, AlphaScalarBranchState, AlphaVectorLeafState,
                AlphaVectorBranchState, ParamsState,
            };

            fn make_state(seed_offset: i64) -> (ParamsState, i64) {
                (
                    ParamsState::$variant($state_ty {
                        params: params($name, $depth, $min_b, $max_b, $min_l, $max_l),
                        essay: None,
                        input_schema: None,
                        essay_tasks: None,
                        tasks: None,
                        tasks_length: None,
                        description: None,
                        readme: None,
                        checker_seed: None,
                    }),
                    ($base_seed as i64) + seed_offset,
                )
            }

            async fn run_snapshot(offset: i64, path: &str, expected: &str) {
                let (state, seed) = make_state(offset);
                let request = make_recursive_request(state, seed);
                let result = normalize_recursive(run_recursive_invention(request).await);
                let json = serde_json::to_string_pretty(&result).unwrap();
                assert_recursive_snapshot(&json, path, expected);
            }

            #[tokio::test] async fn seed_0() {
                run_snapshot(0,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/recursive_client_tests/", $base, "_0.json"),
                    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/recursive_client_tests/", $base, "_0.json"))).await;
            }
            #[tokio::test] async fn seed_1() {
                run_snapshot(1,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/recursive_client_tests/", $base, "_1.json"),
                    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/recursive_client_tests/", $base, "_1.json"))).await;
            }
            #[tokio::test] async fn seed_2() {
                run_snapshot(2,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/recursive_client_tests/", $base, "_2.json"),
                    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/recursive_client_tests/", $base, "_2.json"))).await;
            }
        }
    };
}

#[macro_export]
macro_rules! recursive_test_3x_unrouted {
    (
        $test_name:ident,
        $variant:ident, $state_ty:ident,
        $name:expr, $depth:expr,
        $min_b:expr, $max_b:expr, $min_l:expr, $max_l:expr,
        $base_seed:expr,
        $base:expr
    ) => {
        mod $test_name {
            use $crate::common::inventions::*;
            use objectiveai_sdk::functions::inventions::state::{
                AlphaScalarState, AlphaVectorState, ParamsState,
            };

            fn make_state(seed_offset: i64) -> (ParamsState, i64) {
                (
                    ParamsState::$variant($state_ty {
                        params: params($name, $depth, $min_b, $max_b, $min_l, $max_l),
                        input_schema: None,
                    }),
                    ($base_seed as i64) + seed_offset,
                )
            }

            async fn run_snapshot(offset: i64, path: &str, expected: &str) {
                let (state, seed) = make_state(offset);
                let request = make_recursive_request(state, seed);
                let result = normalize_recursive(run_recursive_invention(request).await);
                let json = serde_json::to_string_pretty(&result).unwrap();
                assert_recursive_snapshot(&json, path, expected);
            }

            #[tokio::test] async fn seed_0() {
                run_snapshot(0,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/recursive_client_tests/", $base, "_0.json"),
                    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/recursive_client_tests/", $base, "_0.json"))).await;
            }
            #[tokio::test] async fn seed_1() {
                run_snapshot(1,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/recursive_client_tests/", $base, "_1.json"),
                    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/recursive_client_tests/", $base, "_1.json"))).await;
            }
            #[tokio::test] async fn seed_2() {
                run_snapshot(2,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/recursive_client_tests/", $base, "_2.json"),
                    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/recursive_client_tests/", $base, "_2.json"))).await;
            }
        }
    };
}
