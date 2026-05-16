//! Error endpoint client for testing error handling.

use futures::Stream;
use objectiveai_sdk::error::{request::ErrorCreateParams, response::ErrorResponse};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// Client for the error endpoint.
pub struct Client;

impl Client {
    pub fn new() -> Self {
        Self
    }

    /// Creates an unary error response. Coin flip determines Ok vs Err.
    pub fn create_unary<CTXEXT>(
        &self,
        _ctx: &crate::ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        body: &ErrorCreateParams,
    ) -> Result<ErrorResponse, objectiveai_sdk::error::ResponseError> {
        let mut rng = make_rng(body.seed);
        if rng.random_bool(0.5) {
            Ok(ErrorResponse { ok: true })
        } else {
            Err(objectiveai_sdk::error::ResponseError {
                code: 500,
                message: serde_json::json!("coin flip error"),
            })
        }
    }

    /// Creates a streaming error response.
    ///
    /// Initial coin flip determines if the stream starts at all (Ok vs Err).
    /// If Ok, returns a stream that yields at least one Ok item, then coin-flips
    /// for each subsequent item until a flip fails.
    pub fn create_streaming<CTXEXT, PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        _ctx: &crate::ctx::Context<CTXEXT, PC>,
        body: &ErrorCreateParams,
    ) -> Result<
        impl Stream<Item = Result<ErrorResponse, objectiveai_sdk::error::ResponseError>> + use<CTXEXT, PC>,
        objectiveai_sdk::error::ResponseError,
    > {
        let mut rng = make_rng(body.seed);
        if rng.random_bool(0.5) {
            return Err(objectiveai_sdk::error::ResponseError {
                code: 500,
                message: serde_json::json!("coin flip error"),
            });
        }
        Ok(async_stream::stream! {
            // Always yield at least one Ok
            yield Ok(ErrorResponse { ok: true });
            loop {
                if rng.random_bool(0.5) {
                    yield Ok(ErrorResponse { ok: true });
                } else {
                    yield Err(objectiveai_sdk::error::ResponseError {
                        code: 500,
                        message: serde_json::json!("coin flip stream error"),
                    });
                    break;
                }
            }
        })
    }
}

fn make_rng(seed: Option<i64>) -> StdRng {
    match seed {
        Some(s) => StdRng::seed_from_u64(s as u64),
        None => StdRng::from_os_rng(),
    }
}
