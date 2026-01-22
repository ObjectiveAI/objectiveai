use crate::{HttpClient, HttpError};

pub async fn get_completion_votes(
    client: &HttpClient,
    id: &str, // vector completion ID
) -> Result<super::response::CompletionVotes, HttpError> {
    client
        .send_unary(
            reqwest::Method::GET,
            &format!("vector/completions/{}", id),
            None::<String>,
        )
        .await
}

pub async fn get_cache_vote(
    client: &HttpClient,
    request: &super::request::CacheVoteRequest<'_>,
) -> Result<super::response::CacheVote, HttpError> {
    client
        .send_unary(
            reqwest::Method::GET,
            "vector/completions/cache",
            Some(request),
        )
        .await
}
